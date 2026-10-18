//! The pad behind a transport: sends the packets of a setting under the cross-process lock.

use std::time::Duration;

use crate::protocol::{Mode, Packet, Settings};
use crate::{Result, Transport};

/// Pause between two reports. G-aim sends without one; the prototype used 10 ms and that worked on the pad.
pub const PACKET_GAP: Duration = Duration::from_millis(10);
/// The pad now and then refuses a SET_FEATURE (Windows error 0x1F, seen during effect changes on the pad);
/// the same report a moment later goes through. Tries per report, and the pause before the n-th retry is n times this.
pub const SEND_ATTEMPTS: u32 = 4;
pub const RETRY_DELAY: Duration = Duration::from_millis(40);

pub struct KmP6<T: Transport> {
    transport: T,
    gap: Duration,
    retry_delay: Duration,
}

impl<T: Transport> KmP6<T> {
    pub fn new(transport: T) -> Self {
        Self { transport, gap: PACKET_GAP, retry_delay: RETRY_DELAY }
    }

    /// without pauses (tests, simulated pad)
    pub fn without_gap(transport: T) -> Self {
        Self { transport, gap: Duration::ZERO, retry_delay: Duration::ZERO }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn into_inner(self) -> T {
        self.transport
    }

    pub fn path(&self) -> String {
        self.transport.path()
    }

    /// Shows the setting on the pad; returns the number of reports sent.
    pub fn apply(&mut self, settings: &Settings) -> Result<usize> {
        let packets = settings.packets()?;
        self.send_all(&packets)?;
        Ok(packets.len())
    }

    /// Switches the LEDs off; the rest of the setting stays as it is for the next `apply`.
    pub fn off(&mut self, settings: &Settings) -> Result<usize> {
        self.apply(&Settings { mode: Mode::Off, ..settings.clone() })
    }

    pub fn send_all(&mut self, packets: &[Packet]) -> Result<()> {
        self.transport.lock()?;
        let result = (|| {
            for (i, p) in packets.iter().enumerate() {
                if i > 0 && !self.gap.is_zero() {
                    std::thread::sleep(self.gap);
                }
                self.send_one(p)?;
            }
            Ok(())
        })();
        self.transport.unlock();
        result
    }

    fn send_one(&mut self, packet: &Packet) -> Result<()> {
        let mut attempt = 1;
        loop {
            match self.transport.set_feature(packet) {
                Err(_) if attempt < SEND_ATTEMPTS => {
                    std::thread::sleep(self.retry_delay * attempt);
                    attempt += 1;
                }
                result => return result,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakePad;
    use crate::protocol::FACTORY_COLORS;

    #[test]
    fn apply_sends_all_packets_and_updates_the_fake() {
        let mut pad = KmP6::without_gap(FakePad::new());
        let s = Settings { mode: Mode::Breathe, cycle: false, slot: 2, ..Settings::default() };
        assert_eq!(pad.apply(&s).unwrap(), 8);
        let fake = pad.transport();
        assert_eq!(fake.sent.len(), 8);
        assert_eq!(fake.colors, FACTORY_COLORS);
        assert_eq!(fake.effect, [0x08, 1, 32, 5, 0x20, 255, 0, 0]);
        assert_eq!(fake.locks, 1);
    }

    #[test]
    fn off_keeps_the_settings() {
        let mut pad = KmP6::without_gap(FakePad::new());
        let s = Settings::default();
        pad.off(&s).unwrap();
        assert_eq!(pad.transport().effect[1..3], [0, 0]);
        assert_eq!(s.mode, Mode::Static);
    }

    #[test]
    fn errors_release_the_lock() {
        let mut pad = KmP6::without_gap(FakePad { fail: true, ..FakePad::new() });
        assert!(pad.apply(&Settings::default()).is_err());
        assert!(!pad.transport().locked);
    }

    #[test]
    fn refused_reports_are_sent_again() {
        let mut pad = KmP6::without_gap(FakePad { refuse: SEND_ATTEMPTS - 1, ..FakePad::new() });
        assert_eq!(pad.apply(&Settings { mode: Mode::Neon, ..Settings::default() }).unwrap(), 1);
        assert_eq!(pad.transport().sent.len(), 1);
        let mut pad = KmP6::without_gap(FakePad { refuse: SEND_ATTEMPTS, ..FakePad::new() });
        assert!(pad.apply(&Settings { mode: Mode::Neon, ..Settings::default() }).is_err());
    }
}
