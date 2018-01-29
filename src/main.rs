extern crate time;
extern crate chrono;
extern crate ws;

use time::precise_time_ns;
use std::thread;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use chrono::prelude::*;
use chrono::offset::LocalResult;
use std::collections::HashMap;
use ws::{listen, connect, Handler, Sender, Result, Message, CloseCode};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::cell::Cell;
use std::cell::RefCell;
use std::vec::Vec;

type RoomNB=u32;
type RoomUsersRegistry = Arc<Mutex<Option<HashMap<RoomNB, Vec<Pair>>>>>;
type UserStatusRegistry = Arc<Mutex<HashMap<u32, bool>>>;

struct Client {
    out: Sender,
    room_id: Option<String>,
    room_nb: RoomNB,
    user_isconnected: UserStatusRegistry,
    room_users: RoomUsersRegistry,
}
struct Pair {
    id: u32,
    out: Sender,
}
impl Clone for Pair {
    fn clone(&self) -> Self {
        Pair {
            id: self.id,
            out: self.out.clone(),
        }
    }
}

struct Server {
    out: Sender,
    count: Rc<Cell<u32>>,
    room_count: Rc<Cell<u32>>,
    room_counter: Rc<RefCell<HashMap<String, u8>>>,
    room_nbs: Rc<RefCell<HashMap<String, u32>>>,
    user_room: Rc<RefCell<HashMap<u32, String>>>,
    user_isconnected: UserStatusRegistry,
    room_users: RoomUsersRegistry,
    id: u32,
    child: Option<std::thread::JoinHandle<()>>,
}
impl Handler for Client {
    fn on_open(&mut self, _: ws::Handshake) -> Result<()> {
        Ok(())
    }
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Ok(mut opt) = self.room_users.lock() {
            if let Some(ref mut hm) = *opt { //open option
                let room_users = hm.get(&self.room_nb);
                if let Some(list) = room_users {
                    for senderpair in list.iter() {
                        let m = msg.to_string().to_owned();

                        let id = senderpair.id;
                        let out = &senderpair.out;
                        //println!("  send to {:?}", id);
                        let hm = self.user_isconnected.lock().unwrap();
                        if let Some(sta) = hm.get(&id) {
                          //  println!("    check status id={:?} {}",id, sta);
                            if *sta {
                                if let Ok(rr) = out.send(m) {
                                    println!("      [{}] send ok", id);
                                } else {
                                    println!("      [{}] senc nok", id);
                                }
                            } else {}
                        } else {
                            println!("  [{:?}] no status",id);
                        }
                    }
                } else {}
                Ok(())
            } else {
                Ok(())
            }
        } else {
            println!("err lock msg");
            Ok(())
        }
    }
}

impl Server {
    fn update_room_count(&mut self, room_id: String) {
        println!("* Update room count");
        let mut a = self.room_counter.borrow_mut();
        let mut co = 0;
        if let Some(rc) = a.get(&room_id) {
            println!("Room has a count {}", rc);
            co = *rc;
        } else {
            println!("Room has no count");
        }
        a.insert(room_id, co);
    }
    fn get_room_nb_by_id(&mut self, room_id: String) -> Option<u32> {
        println!("* Get room nb by id");
        let mut a = self.room_nbs.borrow_mut();

        if let Some(rc) = a.get(&room_id) {
            println!("Room nb for {} is {}", room_id, *rc);
            Some(*rc)
        } else {
            println!("Room has no id");
            None
        }
    }
    fn set_room_nb_by_id(&mut self, room_id: String, room_nb: u32) {
        println!("* Set room nb ");
        let mut a = self.room_nbs.borrow_mut();
        a.insert(room_id, room_nb);
    }
    fn update_user_room(&mut self, room_id: String) {
        //update user room
        println!("* Update user room");
        let mut aa = self.user_room.borrow_mut();
        if let None = aa.get(&self.id) {} else {}
        aa.insert(self.id, room_id);
    }
    fn update_user_isconnected(&mut self) {
        //update user room
        println!("* Update user isconnected SET {} TRUE", self.id);
        let mut aa = self.user_isconnected.lock().unwrap();
        aa.insert(self.id, true);
    }
    fn decrement_room_count(&mut self) {
        println!(" * decrement room_count");
        let mut a = self.user_room.borrow_mut();
        if let Some(room) = a.get(&self.id) {
            let mut B = self.room_counter.borrow_mut();
            let mut has_count = false;
            let mut co = 0;
            if let Some(count) = B.get(room) {
                has_count = true;
                co = *count;
            }
            if has_count {
                B.insert(room.to_string(), co - 1);
            }
        }
    }
    fn update_user_setnotconnected(&mut self) {
        println!("* Update user isconnected");
        let mut aa = self.user_isconnected.lock().unwrap();
        let mut exists = false;
        if let Some(aaa) = aa.get(&self.id) {
            exists = true;
        } else {}
        if exists {
            aa.insert(self.id, false);
        }
    }
    fn update_room_users(&mut self, room_nb: RoomNB) -> bool {
        let mut room_user_in = self.room_users.lock().unwrap();
        let mut exists = false;
        if let Some(ref mut q) = *room_user_in {
            if let Some(qq) = q.get_mut(&room_nb) {
                exists = true
            } else {
                exists = false;
            }
            if (exists) {
                let qq = q.get_mut(&room_nb).unwrap();
                let p = Pair { id: self.id, out: self.out.clone() };
                println!("add user {} to room {}", self.id, room_nb);
                qq.push(p);
            } else {
                let p = Pair { id: self.id, out: self.out.clone() };
                println!("create add user {} to room {}", self.id, room_nb);
                q.insert(room_nb, vec!(p));
            }
        } else {}
        !exists
    }
}

impl Handler for Server {
    fn on_open(&mut self, hs: ws::Handshake) -> Result<()> {
        let path = hs.request.resource();
        let pathsplit: Vec<&str> = path.split("/").collect();
        let broker: &str = pathsplit[1];
        let pair: &str = pathsplit[2];
        let interval: &str = pathsplit[3];
        println!("User {:?}/{:?} connection : broker {} symbol{} interval{}", self.id, self.count, broker, pair, interval);
        println!("Update total count");
        println!("USER ID {}", self.id);


        let url = get_ws_url(broker, pair, interval);

        let room_id = get_ws_id(broker, pair, interval).to_owned();
        self.update_room_count(room_id);

        let room_id = get_ws_id(broker, pair, interval).to_owned();
        self.update_user_room(room_id);

        self.update_user_isconnected();

        let room_id = get_ws_id(broker, pair, interval).to_owned();
        let room_nb_opt = self.get_room_nb_by_id(room_id);
        let mut room_nb:RoomNB;
        if let Some(room_nb_) = room_nb_opt {
            room_nb=room_nb_;
        } else {
            self.room_count.set( self.room_count.get() + 1);
            room_nb=self.room_count.get();
            let room_id = get_ws_id(broker, pair, interval).to_owned();
            self.set_room_nb_by_id(room_id, room_nb);
        }

        let id = self.id;
        let is_room_creation = self.update_room_users(room_nb);
        println!("roomcreation? {}", is_room_creation);
        let user_id = self.count.get();
        let id = Some(get_ws_id(broker, pair, interval).to_owned());
        let w = self.user_isconnected.clone();
        let ww = self.room_users.clone();
        if !is_room_creation {} else {
            println!("  Try connect to exchange {}", url);
            self.child = Some(thread::spawn(move || {
                println!("  New thread {} ", url);
                connect(url, |out2| Client {
                    out: out2,
                    room_id: id.clone(),
                    room_users: ww.clone(),
                    room_nb: room_nb.clone(),
                    user_isconnected: w.clone(),
                }).unwrap();
                println!("  New thread done ");
            }));
        }
        println!("  Tried ");
        self.count.set(self.count.get() + 1);
        self.out.send("{\"wsConnected\":\"true\"")
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("msg {}", msg);
        self.out.send(format!("You are user {} on {:?}", self.id, self.count))
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("The client is done with the connection."),
            CloseCode::Away => {
                println!("The client is leaving the site. Update room count");
                self.update_user_setnotconnected();
                self.out.close(CloseCode::Normal).unwrap();
                //self.decrement_room_count();
            }
            CloseCode::Abnormal => println!("Closing handshake failed! Unable to obtain closing status from client."),
            CloseCode::Protocol => println!("protocol"),
            CloseCode::Unsupported => println!("Unsupported"),
            CloseCode::Status => {
                println!("Status");
                self.update_user_setnotconnected();
                self.out.close(CloseCode::Normal).unwrap();
            },
            CloseCode::Abnormal => println!("Abnormal"),
            CloseCode::Invalid => println!("Invalid"),
            CloseCode::Protocol => println!("protocol"),
            CloseCode::Policy => println!("Policy"),
            CloseCode::Size => println!("Size"),
            CloseCode::Extension => println!("Extension"),
            CloseCode::Protocol => println!("protocol"),
            CloseCode::Restart => println!("Restart"),
            CloseCode::Again => println!("Again"),

            _ => println!("CLOSE The client encountered an error: {}", reason),
        }
    }
    fn on_error(&mut self, err: ws::Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

fn main() {
    println!("Coinamics Server Websockets");
    static WS_PORT: i32 = 3014;
    let count = Rc::new(Cell::new(0));
    let c: u32 = 0;
    let room_count = Rc::new(Cell::new(c));

    let a: HashMap<String, u8> = HashMap::new(); // room id -> number of connected users in the room
    let room_counter = Rc::new(RefCell::new(a));

    let ab: HashMap<String, RoomNB> = HashMap::new();// room id -> room nb
    let room_nbs = Rc::new(RefCell::new(ab));

    let ac: HashMap<u32, String> = HashMap::new(); // user id -> room_id of his room
    let user_room = Rc::new(RefCell::new(ac));

    let ad: HashMap<u32, bool> = HashMap::new();   // room id -> status connected true or false
    let detailed_dispatch: UserStatusRegistry = Arc::new(Mutex::new(ad));

    let ae: HashMap<RoomNB, Vec<Pair>> = HashMap::new();  // room id -> vec of {user id and sender out}
    let room_users: RoomUsersRegistry = Arc::new(Mutex::new(Some(ae)));

    println!("Try listen {}",3014);
    if let Err(error) = listen("0.0.0.0:3014", |out| Server {
        out: out,
        //id: id_counter + 1,
        id: count.get(),
        child: None,
        count: count.clone(),
        room_count: room_count.clone(),
        room_counter: room_counter.clone(),
        room_nbs: room_nbs.clone(),
        user_room: user_room.clone(),
        user_isconnected: detailed_dispatch.clone(),
        room_users: room_users.clone(),
    }) {
        println!("Failed to create WebSocket due to {:?}", error);
    }
}



fn get_ws_url(broker: &str, pair: &str, interval: &str) -> String {
    if broker == "binance" {
        let mut s = "wss://stream.binance.com:9443/ws/".to_owned();
        let pairl = pair.to_lowercase();
        s.push_str(&pairl);
        s.push_str("@kline_");
        s.push_str(&interval);
        s
    } else {
        " ".to_owned()
    }
}

static mut counter: usize = 0;

fn get_ws_id(broker: &str, pair: &str, interval: &str) -> String {
    let mut s = broker.to_string();
    s.push_str("-");
    s.push_str(&pair);
    s.push_str("-");
    s.push_str(&interval);
    s
}

fn get_new_hash(id: u32, val: bool) -> HashMap<u32, bool> {
    let mut A: HashMap<u32, bool> = HashMap::new();
    A.insert(id, val);
    A
}