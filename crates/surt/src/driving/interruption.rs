use std::io;

use surt_engine::runtime::StopSignal;

use crate::message::{Mark, Message};
use crate::symbols::DASH;

#[cfg(unix)]
pub(super) struct Interruptions {
    interrupt: tokio::signal::unix::Signal,
    hangup: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl Interruptions {
    pub(super) fn listen() -> io::Result<Interruptions> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Interruptions {
            interrupt: signal(SignalKind::interrupt())?,
            hangup: signal(SignalKind::hangup())?,
        })
    }

    async fn next(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.hangup.recv() => {}
        }
    }
}

#[cfg(windows)]
pub(super) struct Interruptions {
    ctrl_c: tokio::signal::windows::CtrlC,
    close: tokio::signal::windows::CtrlClose,
}

#[cfg(windows)]
impl Interruptions {
    pub(super) fn listen() -> io::Result<Interruptions> {
        Ok(Interruptions {
            ctrl_c: tokio::signal::windows::ctrl_c()?,
            close: tokio::signal::windows::ctrl_close()?,
        })
    }

    async fn next(&mut self) {
        tokio::select! {
            _ = self.ctrl_c.recv() => {}
            _ = self.close.recv() => {}
        }
    }
}

pub(super) async fn relay(mut interruptions: Interruptions, stop: StopSignal) {
    interruptions.next().await;
    Message::new(format!("stopping {DASH} press Ctrl-C again to force")).print(Mark::Stopped);
    stop.graceful();
    interruptions.next().await;
    stop.force();
}
