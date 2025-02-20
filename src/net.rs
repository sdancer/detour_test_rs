use crate::MitmInfo;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorMove {
    pub id: String,
    pub orig: Vector3,
    pub dest: Vector3,
}

impl ActorMove {
    pub fn new(id: String, orig: Vector3, dest: Vector3) -> Self {
        Self { id, orig, dest }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSpawn {
    pub id: String,
    #[serde(rename = "type")]
    pub actor_type: String,
    pub position: Vector3,
}

impl ActorSpawn {
    pub fn new(id: String, actor_type: String, position: Vector3) -> Self {
        Self {
            id,
            actor_type,
            position,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorDespawn {
    pub id: String,
}

impl ActorDespawn {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

// Define an enum to handle all possible message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum ActorMessage {
    Move(ActorMove),
    Spawn(ActorSpawn),
    Despawn(ActorDespawn),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_message() {
        let move_msg = ActorMessage::Move(ActorMove::new(
            "player1".to_string(),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 10.0),
        ));

        let json = serde_json::to_string(&move_msg).unwrap();
        println!("Move message: {}", json);

        let deserialized: ActorMessage = serde_json::from_str(&json).unwrap();
        if let ActorMessage::Move(msg) = deserialized {
            assert_eq!(msg.id, "player1");
        } else {
            panic!("Wrong message type after deserialization");
        }
    }

    #[test]
    fn test_spawn_message() {
        let spawn_msg = ActorMessage::Spawn(ActorSpawn::new(
            "enemy1".to_string(),
            "goblin".to_string(),
            Vector3::new(5.0, 0.0, 5.0),
        ));

        let json = serde_json::to_string(&spawn_msg).unwrap();
        println!("Spawn message: {}", json);

        let deserialized: ActorMessage = serde_json::from_str(&json).unwrap();
        if let ActorMessage::Spawn(msg) = deserialized {
            assert_eq!(msg.id, "enemy1");
            assert_eq!(msg.actor_type, "goblin");
        } else {
            panic!("Wrong message type after deserialization");
        }
    }

    #[test]
    fn test_despawn_message() {
        let despawn_msg = ActorMessage::Despawn(ActorDespawn::new("enemy1".to_string()));

        let json = serde_json::to_string(&despawn_msg).unwrap();
        println!("Despawn message: {}", json);

        let deserialized: ActorMessage = serde_json::from_str(&json).unwrap();
        if let ActorMessage::Despawn(msg) = deserialized {
            assert_eq!(msg.id, "enemy1");
        } else {
            panic!("Wrong message type after deserialization");
        }
    }
}

pub fn try_read(mitm_info: &mut Arc<MitmInfo>) {
    let a = Arc::get_mut(mitm_info).unwrap();
    if (*a).socket.is_none() {
        return;
    }

    let socket = (*a).socket.as_mut().unwrap();
    let mut lbuf = [0u8; 4];
    let len = socket.peek(&mut lbuf);

    match len {
        Ok(4) => {}
        Ok(_) => return,
        Err(_) => return,
    }

    let _res = socket.read_exact(&mut lbuf);

    //println!("res {:?} {:?}", res, lbuf);

    let count = u32::from_be_bytes(lbuf);
    let mut buf = Vec::with_capacity(count as usize);
    buf.resize(count as usize, 0);
    //println!("reading {:?}", count);

    let _res = socket.read_exact(&mut buf);
    //println!("reading {:?}", buf);

    let text = std::str::from_utf8(&buf).unwrap();

    //println!("read something {}", text);

    let message: ActorMessage = serde_json::from_str(&text).unwrap();

    // Handle different message types
    match message {
        ActorMessage::Move(msg) => println!("Actor {} is moving", msg.id),
        ActorMessage::Spawn(msg) => println!("Spawning {} of type {}", msg.id, msg.actor_type),
        ActorMessage::Despawn(msg) => println!("Despawning {}", msg.id),
    }
}
