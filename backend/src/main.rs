#[macro_use]
extern crate rocket;

use aes_gcm::{
    aead::{
        consts::{B0, B1},
        Aead, KeyInit, OsRng,
    },
    aes::{
        cipher::typenum::{UInt, UTerm},
        Aes256,
    },
    AeadCore, Aes256Gcm, AesGcm, Key,
};
use base64::prelude::*;
use rmp_serde::Serializer;
use rocket::serde::json::Json;
use rocket::{fs::FileServer, State};
use serde::{Deserialize, Serialize};
use std::env;

mod google;
mod key;

#[derive(Serialize)]
struct Time {
    time: String,
}

#[get("/time")]
fn time(claims: google::Claims) -> Json<Time> {
    println!("{:?}", claims);
    Json(Time {
        time: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    dek: String,
    iv: String,
    authorized_users: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedEnvelope {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncodedEnvelope {
    header: String,
}

#[post("/encrypt", data = "<envelope>")]
fn encrypt(envelope: Json<Envelope>, kek: &State<Kek>) -> Json<EncodedEnvelope> {
    let mut buf = Vec::new();
    envelope.serialize(&mut Serializer::new(&mut buf)).unwrap();
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = kek.0.encrypt(&nonce, buf.as_slice()).unwrap();

    let mut header = Vec::new();
    let ee = EncryptedEnvelope {
        nonce: nonce.to_vec(),
        ciphertext: ciphertext.to_vec(),
    };
    ee.serialize(&mut Serializer::new(&mut header)).unwrap();
    let encoded_header = BASE64_STANDARD.encode(&header);
    Json(EncodedEnvelope {
        header: encoded_header,
    })
}

struct Kek(AesGcm<Aes256, UInt<UInt<UInt<UInt<UTerm, B1>, B1>, B0>, B0>>);

#[launch]
fn rocket() -> _ {
    env::set_var("ROCKET_PORT", env::var("PORT").unwrap_or("8000".into()));
    let base64_kek = env::var("KEK").unwrap();
    let bytes_kek = BASE64_STANDARD.decode(base64_kek).unwrap();
    let kek = Key::<Aes256Gcm>::from_slice(&bytes_kek);
    let cipher = Aes256Gcm::new(&kek);
    rocket::build()
        .manage(Kek(cipher))
        .mount("/api", routes![time, encrypt])
        .mount("/", FileServer::from("./static"))
}

#[cfg(test)]
mod tests {
    use super::rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn whatever() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let resp = client.post("/api/encrypt").body("{ \"dek\": \"/eIqTqFEp3GimUezaCO1/R/EKmHgqjQLFX1EWqPknoI=\", \"iv\": \"4+v+r486s6eqknwY\", \"authorized_users\": [\"user1@gmail.com\", \"user2@gmail.com\"] }").dispatch();
        assert!(resp.status() == Status::Ok);
    }
}
