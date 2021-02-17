use async_trait::async_trait;
use tokio::sync::mpsc;

#[cfg(feature = "bounded")]
fn get_channel<MessageType: ActorMessage>() -> (
    impl SendHandle<MessageType>,
    impl ReceiveHandle<MessageType>,
) {
    mpsc::channel(8)
}

#[cfg(not(feature = "bounded"))]
fn get_channel<MessageType: ActorMessage>() -> (
    impl SendHandle<MessageType>,
    impl ReceiveHandle<MessageType>,
) {
    mpsc::unbounded_channel()
}

pub trait ActorMessage: std::fmt::Debug + Send {}
impl<T: std::fmt::Debug + Send> ActorMessage for T {}

#[async_trait]
pub trait ReceiveHandle<MessageType: ActorMessage>: Send {
    async fn recv(&mut self) -> Option<MessageType>;
}

#[async_trait]
impl<MessageType: ActorMessage> ReceiveHandle<MessageType> for mpsc::Receiver<MessageType> {
    async fn recv(&mut self) -> Option<MessageType> {
        self.recv().await
    }
}

#[async_trait]
impl<MessageType: ActorMessage> ReceiveHandle<MessageType>
    for mpsc::UnboundedReceiver<MessageType>
{
    async fn recv(&mut self) -> Option<MessageType> {
        self.recv().await
    }
}

#[async_trait]
pub trait SendHandle<MessageType>: Clone {
    async fn send(&self, value: MessageType) -> Result<(), mpsc::error::SendError<MessageType>>;
}

#[async_trait]
impl<MessageType: ActorMessage> SendHandle<MessageType> for mpsc::Sender<MessageType> {
    async fn send(&self, value: MessageType) -> Result<(), mpsc::error::SendError<MessageType>> {
        self.send(value).await
    }
}

#[async_trait]
impl<MessageType: ActorMessage> SendHandle<MessageType> for mpsc::UnboundedSender<MessageType> {
    async fn send(&self, value: MessageType) -> Result<(), mpsc::error::SendError<MessageType>> {
        self.send(value)
    }
}

pub trait Actor: Send {
    type MessageType: ActorMessage;
    fn handle_message(&mut self, msg: Self::MessageType);
}

pub type MessageTypeFromContainer<T> = <<T as ActorContainer>::ActorType as Actor>::MessageType;

pub trait ActorContainer {
    type ActorType: Actor;
    fn receiver(&mut self) -> &mut dyn ReceiveHandle<MessageTypeFromContainer<Self>>;
    fn actor(&mut self) -> &mut dyn Actor<MessageType = MessageTypeFromContainer<Self>>;
}

pub struct BasicActorContainer<A: Actor, R: ReceiveHandle<A::MessageType>> {
    receiver: R,
    actor: A,
}

impl<A: Actor, R: ReceiveHandle<A::MessageType>> BasicActorContainer<A, R> {
    pub fn new(actor: A, receiver: R) -> Self {
        BasicActorContainer { receiver, actor }
    }
}

impl<A: Actor, R: ReceiveHandle<A::MessageType>> ActorContainer for BasicActorContainer<A, R> {
    type ActorType = A;

    fn receiver(&mut self) -> &mut dyn ReceiveHandle<MessageTypeFromContainer<Self>> {
        &mut self.receiver
    }

    fn actor(&mut self) -> &mut dyn Actor<MessageType = MessageTypeFromContainer<Self>> {
        &mut self.actor
    }
}

pub struct BasicActorHandle<Message: ActorMessage, S: SendHandle<Message>> {
    sender: S,
    _msg: std::marker::PhantomData<Message>,
}

impl<Message: ActorMessage, S: SendHandle<Message>> Clone for BasicActorHandle<Message, S> {
    fn clone(&self) -> Self {
        BasicActorHandle {
            sender: self.sender.clone(),
            _msg: Default::default(),
        }
    }
}

impl<Message: ActorMessage, S: SendHandle<Message>> BasicActorHandle<Message, S> {
    pub fn new<A: 'static + Actor<MessageType = Message>, R: 'static + ReceiveHandle<Message>>(
        actor: A,
        sender: S,
        receiver: R,
    ) -> BasicActorHandle<Message, S> {
        let container = BasicActorContainer::new(actor, receiver);
        tokio::spawn(run_actor(container));

        Self {
            sender,
            _msg: Default::default(),
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.sender.send(msg).await
    }
}

async fn run_actor(mut actor_container: impl ActorContainer) {
    while let Some(msg) = actor_container.receiver().recv().await {
        actor_container.actor().handle_message(msg);
    }
}

pub fn spawn<A: 'static + Actor>(a: A) -> BasicActorHandle<A::MessageType, impl SendHandle<A::MessageType>> {
    let (sender, receiver) = get_channel();
    BasicActorHandle::new(a, sender, receiver)
}

pub fn spawn_default<A: 'static + Default + Actor>() -> BasicActorHandle<A::MessageType, impl SendHandle<A::MessageType>> {
    let (sender, receiver) = get_channel();
    BasicActorHandle::new(A::default(), sender, receiver)
}
