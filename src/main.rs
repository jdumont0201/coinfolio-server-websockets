extern crate time;
extern crate chrono;
extern crate ws;
extern crate serde_json;
extern crate openssl;

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
mod Universal;
mod Server;
mod Client;
use std::env;
use ws::util::TcpStream;
use std::io::Read;
use std::fs::File;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod, SslStream};
use openssl::pkey::PKey;
use openssl::x509::{X509, X509Ref};


type RoomNB = u32;
type RoomUsersRegistry = Arc<Mutex<Option<HashMap<RoomNB, Vec<Pair>>>>>;
type UserStatusRegistry = Arc<Mutex<HashMap<u32, bool>>>;

pub struct Pair {
    pub id: u32,
    pub out: Sender,
}

impl Clone for Pair {
    fn clone(&self) -> Self {
        Pair {
            id: self.id,
            out: self.out.clone(),
        }
    }
}

//return true if user is still connected, false otherwise
fn send_msg_to_user(client: &Client::Client, senderpair: &Pair, msg2: String, room_id: String) -> bool {
    let id = senderpair.id;
    let out = &senderpair.out;
    //println!("  send to {:?}", id);
    let hm = client.user_isconnected.lock().unwrap();
    if let Some(sta) = hm.get(&id) {
        //  println!("    check status id={:?} {}",id, sta);
        if *sta {
            if let Ok(_rr) = out.send(msg2) {
                println!("      [{}] [{}] send ok", room_id, id);
            } else {
                println!("      [{}] senc nok", id);
            }
            true
        } else {
            //println!("      [{}] [{}] user disc", room_id, id);
            false
        }
    } else {
        println!("  [{:?}] no status", id);
        true
    }
}


fn main() {
    println!("Coinamics Server Websockets");
    static WS_PORT: i32 = 3014;

    //CREATE SHARED VARIABLES
    let count = Rc::new(Cell::new(0));
    let c: u32 = 0;
    let room_count = Rc::new(Cell::new(c));

    let a: HashMap<String, u8> = HashMap::new(); // room id -> number of connected users in the room
    let room_counter = Rc::new(RefCell::new(a));

    let ab: HashMap<String, RoomNB> = HashMap::new();// room id -> room nb
    let room_nbs = Rc::new(RefCell::new(ab));

    //let ac: HashMap<u32, String> = HashMap::new(); // user id -> room_id of his room
    //let user_room = Rc::new(RefCell::new(ac));

    let ad: HashMap<u32, bool> = HashMap::new();   // room id -> status connected true or false
    let detailed_dispatch: UserStatusRegistry = Arc::new(Mutex::new(ad));

    let ae: HashMap<RoomNB, Vec<Pair>> = HashMap::new();  // room id -> vec of {user id and sender out}
    let room_users: RoomUsersRegistry = Arc::new(Mutex::new(Some(ae)));

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("not enough arguments");
        return;
    }

    let certpath = &args[1].to_string();
    let keypath = &args[2].to_string();

    println!("CRT: {}",certpath);
    println!("KEY: {}",keypath);
    //READ FIRST FILE
    let read = read_file(certpath);
    match read {
        Ok(read_) => {
            //EXTRACT FIRST FILE
            let crt = X509::from_pem(read_.as_ref());
            match (crt) {
                Ok(crt_) => {
                    println!("read crt ok");

                    //READ 2ND FILE
                    let read2 = read_file(keypath);
                    match read2 {
                        Ok(read2_) => {
                            //EXTRACT 2ND FILE
                            let key = PKey::private_key_from_pem(read2_.as_ref());
                            match (key) {
                                Ok(key_) => {
                                    println!("read key ok");


                                    let acceptor = Rc::new(SslAcceptorBuilder::mozilla_intermediate(
                                        SslMethod::tls(),
                                        &key_,
                                        &crt_,
                                        std::iter::empty::<X509Ref>(),
                                    ).unwrap().build());

                                    println!("acceptor ok");
                                    let serv = ws::Builder::new().with_settings(ws::Settings {
                                        encrypt_server: true,
                                        ..ws::Settings::default()
                                    }).build(|out: ws::Sender| {
                                        Server::Server {
                                            ssl:acceptor.clone(),
                                            out: out,
                                            id: count.get(),
                                            child: None,
                                            count: count.clone(),
                                            room_count: room_count.clone(),
                                            room_counter: room_counter.clone(),
                                            room_nbs: room_nbs.clone(),
                                            //user_room: user_room.clone(),
                                            user_isconnected: detailed_dispatch.clone(),
                                            room_users: room_users.clone(),
                                        }
                                    });
                                    println!("build ok");
                                    match serv {
                                        Ok(serv_) => {
                                            println!("serv ok");
                                            let lis = serv_.listen(format!("0.0.0.0:{}", WS_PORT));
                                            match lis {
                                                Ok(lis_) => {
                                                    println!("listen ok")
                                                }
                                                Err(err) => {
                                                    println!("err listen {:?}", err)
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            println!("cannot build serv {:?}", err)
                                        }
                                    }
                                },
                                Err(err) => {
                                    println!("cannot extract key")
                                }
                            }
                        },
                        Err(err) => {
                            println!("cannot read key")
                        }
                    }
                }
                Err(err) => {
                    println!("cannot extract crt")
                }
            }
        },

        Err(err) => {
            println!("cannot read crt")
        }
    }


    println!("Try listen {}", WS_PORT);
}

fn read_file(name: &str) -> std::io::Result<Vec<u8>> {
    let mut file = try!(File::open(name));
    let mut buf = Vec::new();
    try!(file.read_to_end(&mut buf));
    Ok(buf)
}

fn get_ws_url(broker: &str, pair: &str, interval: &str) -> String {
    if broker == "binance" {
        let mut s = "wss://stream.binance.com:9443/ws/".to_owned();
        let pairl = pair.to_lowercase();
        s.push_str(&pairl);
        s.push_str("@kline_");
        s.push_str(&interval);
        s
    } else if broker == "hitbtc" {
        "wss://api.hitbtc.com/api/2/ws".to_string()
    } else {
        " ".to_owned()
    }
}


fn get_ws_id(broker: &str, pair: &str, interval: &str) -> String {
    let mut s = broker.to_string();
    s.push_str("-");
    s.push_str(&pair);
    s.push_str("-");
    s.push_str(&interval);
    s
}

