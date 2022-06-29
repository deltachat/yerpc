pub struct EventEmitter<T: Clone> {
    rx: async_broadcast::InactiveReceiver<T>,
    tx: async_broadcast::Sender<T>,
}
impl<T: Clone> EventEmitter<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = async_broadcast::broadcast(cap);
        let rx = rx.deactivate();
        Self { tx, rx }
    }
    pub async fn emit(&self, event: T) -> Result<(), async_broadcast::SendError<T>> {
        if self.tx.receiver_count() == 0 {
            return Ok(());
        }
        self.tx.broadcast(event).await?;
        Ok(())
    }
    pub fn subscribe(&self) -> async_broadcast::Receiver<T> {
        self.rx.activate_cloned()
    }
}
