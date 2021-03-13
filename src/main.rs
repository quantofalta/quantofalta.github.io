use std::{env, fs};

use text_io::read;

async fn log_in() {
    let app_secret_json = include_str!("../app-secret.json");
    let con_token: egg_mode::KeyPair = serde_json::from_str(app_secret_json).unwrap();
    // "oob" is needed for PIN-based auth; see docs for `request_token` for more info
    let request_token = egg_mode::auth::request_token(&con_token, "oob")
        .await
        .unwrap();
    let auth_url = egg_mode::auth::authorize_url(&request_token);
    println!("Visit this URL and then type the given PIN: {}", auth_url);
    let verifier: String = read!("{}\n");
    // note this consumes con_token; if you want to sign in multiple accounts, clone it here
    let (token, _user_id, _screen_name) =
        egg_mode::auth::access_token(con_token, &request_token, verifier)
            .await
            .unwrap();
    match token {
        egg_mode::auth::Token::Access { consumer: _, access } => {
            let token_json = serde_json::to_string(&access).unwrap();
            println!("{}", token_json);
        }
        _ => {}
    }
}

fn get_app_key_pair() -> egg_mode::KeyPair {
    let app_secret_json = include_str!("../app-secret.json");
    let token: egg_mode::KeyPair = serde_json::from_str(app_secret_json).unwrap();
    return token;
}

fn get_token() -> egg_mode::Token {
    let app_key_pair = get_app_key_pair();
    let user_secret_json = fs::read_to_string("user-secret.json").unwrap();
    let user_key_pair: egg_mode::KeyPair = serde_json::from_str(&user_secret_json).unwrap();
    return egg_mode::Token::Access{access: user_key_pair, consumer: app_key_pair};
}

async fn post_tweet() {
    let token = get_token();

    use egg_mode::tweet::DraftTweet;

    let _post = DraftTweet::new("Hey Twitter!").send(&token).await.unwrap();
}

// fn test_json() {
//     let app_secret_json = include_str!("../app-secret.json");
//     let token: egg_mode::KeyPair = serde_json::from_str(app_secret_json).unwrap();
//     let j = serde_json::to_string(&token).unwrap();
//     println!("{}", j);
// }

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        if args[1] == "login" {
            log_in().await;
            return;
        }
    }
    post_tweet().await;
}
