use anyhow::{anyhow, Result};
use date_component::date_component;

use serde;
use std::{env, fs};
use text_io;

const DATA_URL: &str = "https://covid.ourworldindata.org/data/vaccinations/vaccinations.csv";

async fn log_in() {
    let app_secret_json = include_str!("../app-secret.json");
    let con_token: egg_mode::KeyPair = serde_json::from_str(app_secret_json).unwrap();
    // "oob" is needed for PIN-based auth; see docs for `request_token` for more info
    let request_token = egg_mode::auth::request_token(&con_token, "oob")
        .await
        .unwrap();
    let auth_url = egg_mode::auth::authorize_url(&request_token);
    println!("Visit this URL and then type the given PIN: {}", auth_url);
    let verifier: String = text_io::read!("{}\n");
    // note this consumes con_token; if you want to sign in multiple accounts, clone it here
    let (token, _user_id, _screen_name) =
        egg_mode::auth::access_token(con_token, &request_token, verifier)
            .await
            .unwrap();
    match token {
        egg_mode::auth::Token::Access {
            consumer: _,
            access,
        } => {
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
    return egg_mode::Token::Access {
        access: user_key_pair,
        consumer: app_key_pair,
    };
}

async fn post_tweet() {
    let token = get_token();

    use egg_mode::tweet::DraftTweet;

    let _post = DraftTweet::new("Hey Twitter!").send(&token).await.unwrap();
}

async fn download_data() -> Result<String> {
    let response = reqwest::get(DATA_URL).await?;
    let text = response.text().await?;
    Ok(text)
}

#[derive(Debug, serde::Deserialize)]
struct Record {
    location: String,
    iso_code: String,
    date: String,
    total_vaccinations: Option<u32>,
    people_vaccinated: Option<u32>,
    people_fully_vaccinated: Option<u32>,
    daily_vaccinations_raw: Option<u32>,
    daily_vaccinations: Option<u32>,
    total_vaccinations_per_hundred: Option<f32>,
    people_vaccinated_per_hundred: Option<f32>,
    people_fully_vaccinated_per_hundred: Option<f32>,
    daily_vaccinations_per_million: Option<u32>,
}

fn get_last_vaccination_data(csv_text: &str, country: &str) -> Result<Record> {
    let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
    let mut last_daily_vaccinations: Option<Record> = None;
    for result in rdr.deserialize() {
        let record: Record = match result {
            Ok(r) => r,
            Err(e) => {
                println!("{}", e);
                continue
            },
        };
        let c = &record.location;
        if c == country {
            last_daily_vaccinations = Some(record);
        }
    }
    Ok(last_daily_vaccinations.ok_or(anyhow!("No daily vaccinations found"))?)
}

fn get_brazil_immunization_estimate(total_vaccinations: u32, daily_vaccinations: u32) -> chrono::Duration {
    // https://ftp.ibge.gov.br/Estimativas_de_Populacao/Estimativas_2020/POP2020_20210204.pdf
    const BRAZIL_POPULATION: u32 = 211755692;
    let herd_size = (BRAZIL_POPULATION * 7) / 10;
    let doses = std::cmp::max(herd_size * 2 - total_vaccinations, 0);
    let days = doses / daily_vaccinations;
    return chrono::Duration::days(days.into());
}

fn format_estimate(now: chrono::DateTime<chrono::Utc>, estimate: chrono::Duration) -> String {
    let end = now + estimate;
    let components = date_component::calculate(&now, &end);
    let mut vec: Vec<String> = Vec::new();
    if components.year > 0 {
        vec.push(format!(
            "{} {}",
            components.year,
            if components.year == 1 { "ano" } else { "anos" }
        ));
    }
    if components.month > 0 {
        vec.push(format!(
            "{} {}",
            components.month,
            if components.month == 1 {
                "mês"
            } else {
                "meses"
            }
        ));
    }
    if components.day > 0 {
        vec.push(format!(
            "{} {}",
            components.day,
            if components.day == 1 { "dia" } else { "dias" }
        ));
    }
    let s = match vec.len() {
        1 => vec[0].clone(),
        2 => vec.as_slice().join(" e "),
        3 => format!("{}, {} e {}", vec[0], vec[1], vec[2]),
        _ => unreachable!()
    };
    if vec.len() == 1 && &vec[0][0..1] == "1" {
        return format!("falta {}", s);
    }
    return format!("faltam {}", s);
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
    // post_tweet().await;
    // let r = download_data().await.unwrap();
    let r = fs::read_to_string("data.csv").unwrap();
    println!("{}", get_last_vaccination_data(&r, "Brazil").unwrap().daily_vaccinations.unwrap());
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn get_last_vaccination_data_works() {
        let test_csv = include_str!("./testdata/test.csv");
        let d = get_last_vaccination_data(&test_csv, "Brazil").unwrap();
        assert_eq!(d.daily_vaccinations.unwrap(), 168025);
    }

    #[test]
    fn get_brazil_immunization_estimate_works() {
        let e = get_brazil_immunization_estimate(0, 168025);
        assert_eq!(e, chrono::Duration::days(1764));
        let e = get_brazil_immunization_estimate(11422666, 168025);
        assert_eq!(e, chrono::Duration::days(1696));
    }

    #[test]
    fn format_estimate_works() {
        let start = chrono::Utc.ymd(2021, 3, 16).and_hms(0, 0, 0);
        let r = format_estimate(start, chrono::Duration::days(1));
        assert_eq!(r, "falta 1 dia");
        let r = format_estimate(start, chrono::Duration::days(2));
        assert_eq!(r, "faltam 2 dias");
        let r = format_estimate(start, chrono::Duration::days(31));
        assert_eq!(r, "falta 1 mês");
        let r = format_estimate(start, chrono::Duration::days(32));
        assert_eq!(r, "faltam 1 mês e 1 dia");
        let r = format_estimate(start, chrono::Duration::days(33));
        assert_eq!(r, "faltam 1 mês e 2 dias");
        let r = format_estimate(start, chrono::Duration::days(31 + 30));
        assert_eq!(r, "faltam 2 meses");
        let r = format_estimate(start, chrono::Duration::days(365));
        assert_eq!(r, "falta 1 ano");
        let r = format_estimate(start, chrono::Duration::days(365 + 31));
        assert_eq!(r, "faltam 1 ano e 1 mês");
        let r = format_estimate(start, chrono::Duration::days(365 + 31 + 1));
        assert_eq!(r, "faltam 1 ano, 1 mês e 1 dia");
        let r = format_estimate(start, chrono::Duration::days(365 * 2 + 31 + 1));
        assert_eq!(r, "faltam 2 anos, 1 mês e 1 dia");
    }
}
