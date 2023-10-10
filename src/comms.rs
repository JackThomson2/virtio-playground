use tokio::sync::mpsc::{channel, Sender, Receiver};

pub enum Messages {
    String(String),
    FileWrite(String, String),
}

pub struct CommsLink {
    pub tx: Sender<Messages>,
    pub rx: Receiver<Messages>,
}

impl CommsLink {
    pub fn new_pair() -> (Self, Self) {
        let (tx1, rx1) = channel(100);
        let (tx2, rx2) = channel(100);

        (Self::from_channels(tx1, rx2), Self::from_channels(tx2, rx1))
    }

    pub fn from_channels(tx: Sender<Messages>, rx: Receiver<Messages>) -> Self {
        Self {
            tx,
            rx
        }
    }
}
