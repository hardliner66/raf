use raf::*;
use tokio::sync::oneshot;

#[derive(Default)]
struct MyActor {
    next_id: u32,
}

impl Actor for MyActor {
    type MessageType = MyActorMessage;

    fn handle_message(&mut self, msg: Self::MessageType) {
        match msg {
            MyActorMessage::GetUniqueId { respond_to } => {
                self.next_id += 1;
                respond_to.send(self.next_id).unwrap();
            }
        }
    }
}

#[derive(Debug)]
enum MyActorMessage {
    GetUniqueId { respond_to: oneshot::Sender<u32> },
}

#[tokio::main]
async fn main() {
    let actor = spawn_default::<MyActor>();
    let actor2 = actor.clone();
    for _ in 0..10_usize {
        let (sender, receiver) = oneshot::channel();
        let msg = MyActorMessage::GetUniqueId { respond_to: sender };
        let _ = actor.send(msg).await;
        let id = receiver.await.unwrap();
        println!("id: {}", id);
    }
    let (sender, receiver) = oneshot::channel();
    let msg = MyActorMessage::GetUniqueId { respond_to: sender };
    let _ = actor2.send(msg).await;
    let id = receiver.await.unwrap();
    println!("id: {}", id);
}
