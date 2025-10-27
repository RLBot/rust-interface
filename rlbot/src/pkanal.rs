use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Sender<T> {
    internal: kanal::Sender<T>,
    waker: Arc<mio::Waker>,
}

impl<T> Sender<T> {
    pub fn send(&self, msg: T) -> Result<(), kanal::SendError> {
        self.internal.send(msg)?;
        self.waker.wake().expect("couldn't wake pkanal waker");
        Ok(())
    }

    pub fn drop_and_wake(self) {
        drop(self.internal);
        self.waker.wake().expect("couldn't wake pkanal waker");
    }
}

#[derive(Debug, Clone)]
pub struct Receiver<T> {
    internal: kanal::Receiver<T>,
}

impl<T> Receiver<T> {
    // pub fn recv(&self) -> Result<T, kanal::ReceiveError> {
    //     self.internal.recv()
    // }
    pub fn try_recv(&self) -> Result<Option<T>, kanal::ReceiveError> {
        self.internal.try_recv()
    }
}

pub fn unbounded<T>(registry: &mio::Registry, token: mio::Token) -> (Sender<T>, Receiver<T>) {
    let (ks, kr) = kanal::unbounded::<T>();
    let waker = mio::Waker::new(registry, token).expect("couldn't create pkanal waker");
    (
        Sender {
            internal: ks,
            waker: Arc::new(waker),
        },
        Receiver { internal: kr },
    )
}
