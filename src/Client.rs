use ws;

use RoomNB;
use UserStatusRegistry;
use RoomUsersRegistry;
use send_msg_to_user;
use ws::{listen, connect, Handler, Sender, Result, Message, CloseCode};
use Universal;
pub struct Client {
    pub out: Sender,
    pub room_id: Option<String>,
    pub room_nb: RoomNB,
    pub pair: String,
    pub broker: String,
    pub user_isconnected: UserStatusRegistry,
    pub room_users: RoomUsersRegistry,
    pub oldp: String,
    pub oldv: String,
}


impl Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> Result<()> {
        if self.broker == "hitbtc" {
            let json = format!("{{ \"method\": \"subscribeCandles\",\"params\": {{\"symbol\": \"{}\",\"period\":\"M1\"}},\"id\": 123 }}", self.pair);
            println!("{} {} ", self.broker, json);
            self.out.send(json)
        } else {
            Ok(())
        }
    }
    fn on_message(&mut self, msg: Message) -> Result<()> {
        let m = msg.to_string().to_owned();
        let message: Option<String> = Universal::get_universal_msg(self, &m);
        match message {
            Some(message_) => {
                let mm = message_.clone();
                let room_id = self.room_id.clone().unwrap();
                let mut clearIds: Vec<usize> = Vec::new();
                if let Ok(mut opt) = self.room_users.lock() {
                    if let Some(ref mut hm) = *opt { //open option
                        let mut room_users = hm.get_mut(&self.room_nb);
                        if let Some(mut list) = room_users {
                            //run send msg and keep only items where send_msg=true (i.e. user still connected)
                            list.retain(|&ref x| send_msg_to_user(self, x, mm.clone(), room_id.clone()));
                        }
                    } else {}
                    Ok(())
                } else {
                    Ok(())
                }
            }
            None => {
                Ok(())
            }
        }
    }
}
