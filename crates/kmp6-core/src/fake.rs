//! Simulated pad for tests and the GUI dry run: records every report and keeps what the pad would show.

use crate::protocol::{CMD_COLORS, CMD_EFFECT, FACTORY_COLORS, Packet, Rgb, SLOTS};
use crate::transport::{REPORT_LEN, Transport};
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct FakePad {
    pub sent: Vec<Packet>,
    /// colours of the slots as the pad has them
    pub colors: [Rgb; SLOTS],
    /// last effect report (`08 …`)
    pub effect: Packet,
    /// every send fails (unplugged pad)
    pub fail: bool,
    /// the next sends fail, then they go through again (the pad refusing a report for a moment)
    pub refuse: u32,
    pub locked: bool,
    pub locks: usize,
}

impl Default for FakePad {
    fn default() -> Self {
        Self {
            sent: Vec::new(),
            colors: FACTORY_COLORS,
            effect: [CMD_EFFECT, 0, 32, 0, 0x70, 255, 0, 0],
            fail: false,
            refuse: 0,
            locked: false,
            locks: 0,
        }
    }
}

impl FakePad {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for FakePad {
    fn set_feature(&mut self, data: &[u8; REPORT_LEN]) -> Result<()> {
        if self.fail {
            return Err(Error::Transport("simulated pad unplugged".into()));
        }
        if self.refuse > 0 {
            self.refuse -= 1;
            return Err(Error::Transport("simulated pad busy".into()));
        }
        self.sent.push(*data);
        match data[0] {
            CMD_EFFECT => {
                self.effect = *data;
                self.colors[0] = [data[5], data[6], data[7]];
            }
            cmd => {
                let Some(i) = CMD_COLORS.iter().position(|&c| c == cmd) else {
                    panic!("unexpected report {}", crate::hex(data));
                };
                self.colors[1 + 2 * i] = [data[1], data[2], data[3]];
                self.colors[2 + 2 * i] = [data[4], data[5], data[6]];
            }
        }
        Ok(())
    }

    fn lock(&mut self) -> Result<()> {
        self.locked = true;
        self.locks += 1;
        Ok(())
    }

    fn unlock(&mut self) {
        self.locked = false;
    }

    fn path(&self) -> String {
        "simulated".into()
    }
}
