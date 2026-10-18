//! All pad access runs in a background thread; the window never blocks.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use kmp6_core::fake::FakePad;
use kmp6_core::{KmP6, Settings, Transport, config, open_transport};

pub type Pad = KmP6<Box<dyn Transport>>;

pub enum Job {
    /// send the setting and store it
    Apply(Settings),
    /// only look whether the pad is there
    Connect,
}

pub struct Done {
    pub job: Job,
    pub result: Result<usize, String>,
    /// the pad is open after the job (an error closes it until the next job)
    pub connected: bool,
}

pub struct Device {
    tx: Sender<Job>,
    pub rx: Receiver<Done>,
    /// queued + running, only used in the GUI thread
    pub pending: usize,
}

impl Device {
    pub fn spawn(ctx: eframe::egui::Context, device: Option<String>, fake: bool) -> Self {
        let (tx, jobs) = channel::<Job>();
        let (done_tx, rx) = channel::<Done>();
        thread::spawn(move || {
            let mut pad: Option<Pad> = None;
            for job in jobs {
                let result = (|| {
                    if pad.is_none() {
                        let t: Box<dyn Transport> =
                            if fake { Box::new(FakePad::new()) } else { open_transport(device.as_deref())? };
                        pad = Some(if fake { KmP6::without_gap(t) } else { KmP6::new(t) });
                    }
                    let p = pad.as_mut().expect("connected");
                    match &job {
                        Job::Apply(s) => {
                            let n = p.apply(s)?;
                            if !fake {
                                config::save(s)?;
                            }
                            Ok(n)
                        }
                        Job::Connect => Ok(0),
                    }
                })();
                if result.is_err() && !fake {
                    pad = None; // reconnect on the next job
                }
                let done =
                    Done { job, result: result.map_err(|e: kmp6_core::Error| e.to_string()), connected: pad.is_some() };
                if done_tx.send(done).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { tx, rx, pending: 0 }
    }

    pub fn submit(&mut self, job: Job) {
        self.pending += 1;
        // The worker lives as long as the window; an error here would mean it crashed.
        if self.tx.send(job).is_err() {
            self.pending -= 1;
        }
    }

    pub fn idle(&self) -> bool {
        self.pending == 0
    }
}
