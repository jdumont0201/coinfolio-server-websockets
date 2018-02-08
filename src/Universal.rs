

    use serde_json;
    use chrono::TimeZone;
    use Client::Client;
    pub struct GenericTick {
        ts: i64,
        p: f64,
        v: f64,
    }

    impl GenericTick {
        fn to_string(&self) -> String {
            let mut owned_str: String = "".to_owned();
            owned_str.push_str(&(self.ts.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.p.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.v.to_string().to_owned()));
            owned_str.push_str("\n");
            owned_str
        }
        fn to_json(&self, pair: &str) -> String {
            let ts = super::chrono::Utc.timestamp(self.ts / 1000, 0).format("%Y-%m-%d %H:%M:%S");
            let s = format!(r#"{{"ts" :"{}","pair"  :"{}","close" :"{}","volume":"{}"}}"#, ts, pair, self.p, self.v);
            s
        }
    }

    pub struct StringGenericTick {
        ts: String,
        p: String,
        v: String,
    }

    impl StringGenericTick {
        fn to_string(&self) -> String {
            let mut owned_str: String = "".to_owned();
            owned_str.push_str(&(self.ts.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.p.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.v.to_string().to_owned()));
            owned_str.push_str("\n");
            owned_str
        }
        fn to_json(&self) -> String {
            let s = format!(r#"{{"ts" :{},"p" :{},"v":{}}}"#, self.ts, self.p, self.v);
            s
        }
    }

    pub struct StringGenericOHLC {
        ts: String,
        o: String,
        h: String,
        c: String,
        l: String,
        v: String,
    }

    impl StringGenericOHLC {
        fn to_json(&self) -> String {
            //let ts = chrono::Utc.timestamp(self.ts.timestamp() / 1000, 0).format("%Y-%m-%d %H:%M:%S");
            let n = self.ts.len() - 3;
            let t = self.ts[..n].to_string();
            let s = format!(r#"{{"ts":"{}","o":{},"h":{},"l":{},"c":{},"v":{}}}"#, t, self.o, self.h, self.l, self.c, self.v);
            s
        }
        fn to_string(&self) -> String {
            let mut owned_str: String = "".to_owned();
            owned_str.push_str(&(self.ts.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.o.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.h.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.l.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.c.to_string()).to_owned());
            owned_str.push_str(",");
            owned_str.push_str(&(self.v.to_string().to_owned()));
            owned_str.push_str("\n");
            owned_str
        }
    }



    enum Value {
        String(String),
    }

    pub fn get_universal_msg(client: &mut Client, rawmsg: &String) -> Option<String> {
        let broker = &client.broker;
        if broker == "binance" {
            let tick: serde_json::Value = serde_json::from_str(&rawmsg).unwrap();
            let ts = tick["k"]["t"].to_string();
            let c = tick["k"]["c"].to_string();
            let v = tick["k"]["v"].to_string();
            let o = tick["k"]["o"].to_string();
            let h = tick["k"]["h"].to_string();
            let l = tick["k"]["l"].to_string();
            client.oldp = c;
            client.oldv = v;
            let c = tick["k"]["c"].to_string();
            let v = tick["k"]["v"].to_string();
            //let tsi: i64 = tss.timestamp() * 1000;
            //let ts= tsi.to_string();

            Some(StringGenericOHLC { ts: ts, o: o, l: l, h: h, c: c, v: v }.to_json())
        } else if broker == "hitbtc" {
            let tick: serde_json::Value = serde_json::from_str(&rawmsg).unwrap();
            if tick["result"].to_string() == "true" {
                None
            } else if tick["method"] == "snapshotCandles" {
                None
            } else if tick["method"] == "updateCandles" {

                //convert timestamp to int format
                let mut tsstr = tick["params"]["data"][0]["timestamp"].to_string();

                tsstr = tsstr[1..tsstr.len() - 1].to_string();


                let tss: super::chrono::DateTime<super::chrono::Utc> = tsstr.parse::<super::chrono::DateTime<super::chrono::Utc>>().unwrap();
                let tsi: i64 = tss.timestamp() * 1000;
                let ts = tsi.to_string();

                let c = tick["params"]["data"][0]["close"].to_string();
                let o = tick["params"]["data"][0]["open"].to_string();
                let h = tick["params"]["data"][0]["max"].to_string();
                let l = tick["params"]["data"][0]["min"].to_string();
                let v = tick["params"]["data"][0]["volume"].to_string();
                if client.oldp != c || client.oldv != v {
                    let c = tick["params"]["data"][0]["close"].to_string();
                    let v = tick["params"]["data"][0]["volume"].to_string();
                    client.oldp = c;
                    client.oldv = v;
                    let c = tick["params"]["data"][0]["close"].to_string();
                    let v = tick["params"]["data"][0]["volume"].to_string();
                    Some(StringGenericOHLC { ts: ts, o: o, l: l, h: h, c: c, v: v }.to_json())
                } else {
                    client.oldp = c;
                    client.oldv = v;
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

