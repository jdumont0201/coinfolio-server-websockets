use std;
use ws;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod, SslStream};
use openssl::pkey::PKey;
use openssl::x509::{X509, X509Ref};
use ws::util::TcpStream;
use Pair;
use get_ws_id;
use get_ws_url;
use RoomUsersRegistry;
use UserStatusRegistry;
use RoomNB;
use std::thread;
use chrono::TimeZone;
use std::collections::HashMap;
use ws::{listen, connect, Handler, Sender, Result, Message, CloseCode};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::cell::Cell;
use std::cell::RefCell;
use std::vec::Vec;
use Client::Client;

pub struct Server {
    pub out: Sender,
    pub count: Rc<Cell<u32>>,
    pub room_count: Rc<Cell<u32>>,
    pub room_counter: Rc<RefCell<HashMap<String, u8>>>,
    pub room_nbs: Rc<RefCell<HashMap<String, u32>>>,
    pub user_isconnected: UserStatusRegistry,
    pub room_users: RoomUsersRegistry,
    pub id: u32,
    pub child: Option<std::thread::JoinHandle<()>>,
    pub ssl: Rc<SslAcceptor>,
}

impl Handler for Server {
    fn upgrade_ssl_server(&mut self, sock: TcpStream) -> ws::Result<SslStream<TcpStream>> {
        self.ssl.accept(sock).map_err(From::from)
    }
    fn on_open(&mut self, hs: ws::Handshake) -> Result<()> {
        let path = hs.request.resource();
        let pathsplit: Vec<&str> = path.split("/").collect();
        if pathsplit.len() < 3 {
            println!("Not enough arguments");
            self.out.close(CloseCode::Normal)
        } else {
            let broker: &str = pathsplit[1];
            let pair: &str = pathsplit[2];
            let interval: &str = pathsplit[3];
            println!("+ User {:?}/{:?} connection : broker={} symbol={} interval={}", self.id, self.count, broker, pair, interval);
            println!("  Update total count");
            println!("  USER ID {}", self.id);
            let url = get_ws_url(broker, pair, interval);

            //update shared structures
            let room_id = get_ws_id(broker, pair, interval).to_owned();
            self.update_room_count(room_id);
            let room_id = get_ws_id(broker, pair, interval).to_owned();
            self.update_user_isconnected();
            let room_id = get_ws_id(broker, pair, interval).to_owned();
            println!("room {}", room_id);
            let room_nb: RoomNB = self.set_room_nb(broker.to_owned(), pair.to_owned(), interval.to_owned(), room_id);


            let is_room_creation = self.update_room_users(room_nb);
            println!("  roomcreation? {}", is_room_creation);
            let id = Some(get_ws_id(broker, pair, interval).to_owned());
            let w = self.user_isconnected.clone();
            let ww = self.room_users.clone();
            let b = broker.clone().to_owned();
            let p = pair.clone().to_owned();
            if !is_room_creation {} else {
                println!("  Try connect to exchange {}", url);
                self.child = Some(thread::spawn(move || {
                    println!("  New thread {} ", url);
                    connect(url, |out2| Client {
                        out: out2,
                        broker: b.clone(),
                        pair: p.clone(),
                        room_id: id.clone(),
                        oldp: "".to_string(),
                        oldv: "".to_string(),
                        room_users: ww.clone(),
                        room_nb: room_nb.clone(),
                        user_isconnected: w.clone(),
                    }).unwrap();
                    println!("  New thread done ");

                }));
            }
            self.count.set(self.count.get() + 1);
            self.out.send("{\"wsConnected\":\"true\"}")
        }
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
            }
            CloseCode::Abnormal => println!("Closing handshake failed! Unable to obtain closing status from client."),
            CloseCode::Protocol => println!("protocol"),
            CloseCode::Unsupported => println!("Unsupported"),
            CloseCode::Status => {
                println!("Status");
                self.update_user_setnotconnected();
                self.out.close(CloseCode::Normal).unwrap();
            }
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

impl Server {
    fn update_room_count(&mut self, room_id: String) {
        println!("  * Update room count");
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
        println!("  * Get room nb by id");
        let a = self.room_nbs.borrow_mut();

        if let Some(rc) = a.get(&room_id) {
            println!("Room nb for {} is {}", room_id, *rc);
            Some(*rc)
        } else {
            println!("Room has no id");
            None
        }
    }
    fn set_room_nb_by_id(&mut self, room_id: String, room_nb: u32) {
        println!("  * Set room nb ");
        let mut a = self.room_nbs.borrow_mut();
        a.insert(room_id, room_nb);
    }
    fn update_user_isconnected(&mut self) {
        //update user room
        println!("  * Update user isconnected SET {} TRUE", self.id);
        let mut aa = self.user_isconnected.lock().unwrap();
        aa.insert(self.id, true);
    }
    fn update_user_setnotconnected(&mut self) {
        println!("  * Update user isconnected");
        let mut aa = self.user_isconnected.lock().unwrap();
        let mut exists = false;
        if let Some(_aaa) = aa.get(&self.id) {
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
            if let Some(_qq) = q.get_mut(&room_nb) {
                exists = true
            } else {
                exists = false;
            }
            if exists {
                let qq = q.get_mut(&room_nb).unwrap();
                let p = Pair { id: self.id, out: self.out.clone() };
                println!("  add user {} to room {}", self.id, room_nb);
                qq.push(p);
            } else {
                let p = Pair { id: self.id, out: self.out.clone() };
                println!("  create add user {} to room {}", self.id, room_nb);
                q.insert(room_nb, vec!(p));
            }
        } else {}
        !exists
    }
    fn set_room_nb(&mut self, broker: String, pair: String, interval: String, room_id: String) -> RoomNB {
        let room_nb_opt = self.get_room_nb_by_id(room_id);
        let room_nb: RoomNB;
        if let Some(room_nb_) = room_nb_opt {
            room_nb = room_nb_;
        } else {
            self.room_count.set(self.room_count.get() + 1);
            room_nb = self.room_count.get();
            let room_id = get_ws_id(&broker, &pair, &interval).to_owned();
            self.set_room_nb_by_id(room_id, room_nb);
        }
        room_nb
    }
}

