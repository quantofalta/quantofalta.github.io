use anyhow::{anyhow, Result};
use date_component::date_component;

use std::{env, fs};

const BRAZIL_POPULATION: u32 = 211755692;

const DATA_URL: &str =
    "https://raw.githubusercontent.com/wcota/covid19br/master/cases-brazil-states.csv";

/// Log in into Twitter and generate an authentication token.
async fn log_in() {
    let con_token = get_app_key_pair().unwrap();
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
    if let egg_mode::auth::Token::Access {
        consumer: _,
        access,
    } = token
    {
        let token_json = serde_json::to_string(&access).unwrap();
        println!("{}", token_json);
    }
}

/// Get application key pair from an environment variable or from a file.
fn get_app_key_pair() -> Result<egg_mode::KeyPair> {
    let mut app_secret_json = env::var("QUANTOFALTA_APPSECRET").ok();
    if app_secret_json.is_none() {
        app_secret_json = Some(fs::read_to_string("app-secret.json")?);
    }
    let token: egg_mode::KeyPair = serde_json::from_str(&app_secret_json.unwrap())?;
    Ok(token)
}

/// Get user token using a key pair from an environment variable or from a file.
fn get_token() -> Result<egg_mode::Token> {
    let app_key_pair = get_app_key_pair()?;
    let mut user_secret_json = env::var("QUANTOFALTA_USERSECRET").ok();
    if user_secret_json.is_none() {
        user_secret_json = Some(fs::read_to_string("user-secret.json")?);
    }
    let user_key_pair: egg_mode::KeyPair = serde_json::from_str(&user_secret_json.unwrap())?;
    Ok(egg_mode::Token::Access {
        access: user_key_pair,
        consumer: app_key_pair,
    })
}

/// Generate HTML page from the template.
fn gen_html(estimate: &str) -> Result<()> {
    let template = fs::read_to_string("index.html")?;
    // TODO: improve this. This is pretty ugly...
    let mut formatted_estimate = estimate.replace("faltam ", "faltam <br/><b>");
    formatted_estimate = formatted_estimate.replace("falta ", "faltam <br/><b>");
    formatted_estimate = formatted_estimate.replace(" para ", "</b><br/> para ");
    let html = template.replace("{{estimate}}", &formatted_estimate);
    fs::write("html/index.html", html)?;
    Ok(())
}

/// Post estimate via Twitter and HTML page.
///
/// `print`: indicates that it should print to stdout and not tweet.
async fn post_estimate(print: bool) -> Result<()> {
    let csv_text = download_data().await?;
    let now = chrono::Utc::now();
    let data = get_last_vaccination_data_covid19br(&csv_text, now)?;
    let estimate = get_brazil_immunization_estimate(data.1, data.2);
    let estimate = format_full_estimate(now, estimate);
    let progress = format_progress(&data.0)?;

    gen_html(&estimate)?;

    let estimate_and_progress = format!("{}\n\n{}", estimate, progress);

    if print {
        println!("{}", estimate_and_progress);
    } else {
        let token = get_token()?;
        let _post = egg_mode::tweet::DraftTweet::new(estimate_and_progress)
            .send(&token)
            .await
            .unwrap();
    }

    return Ok(());
}

/// Download vacciation data CSV from Our World in Data.
async fn download_data() -> Result<String> {
    let response = reqwest::get(DATA_URL).await?;
    let text = response.text().await?;
    Ok(text)
}

/// Record type for vaccination data from Our World in Data.
#[derive(Debug, serde::Deserialize)]
struct Record {
    location: String,
    iso_code: String,
    date: String,
    total_vaccinations: Option<u32>,
    people_vaccinated: Option<u32>,
    people_fully_vaccinated: Option<u32>,
    daily_vaccinations_raw: Option<i32>,
    daily_vaccinations: Option<u32>,
    total_vaccinations_per_hundred: Option<f32>,
    people_vaccinated_per_hundred: Option<f32>,
    people_fully_vaccinated_per_hundred: Option<f32>,
    daily_vaccinations_per_million: Option<u32>,
}

/// Get last vacciation data from Our World in Data for the given country using
/// the previously downloaded CSV.
#[allow(dead_code)]
fn get_last_vaccination_data(csv_text: &str, country: &str) -> Result<Record> {
    let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
    let mut last_record: Option<Record> = None;
    for result in rdr.deserialize() {
        let record: Record = match result {
            Ok(r) => r,
            Err(e) => {
                log::debug!("Error parsing record: {:?}", e);
                continue;
            }
        };
        let c = &record.location;
        if c == country {
            last_record = Some(record);
        }
    }
    log::debug!("Last record: {:?}", last_record);
    last_record.ok_or_else(|| anyhow!("No daily vaccinations found"))
}

/// Record type for vaccination data from covid19br.
#[derive(Debug, Clone, serde::Deserialize)]
struct RecordCovid19br {
    // epi_week: u32,
    date: String,
    country: String,
    state: String,
    // city: String,
    // #[serde(rename = "newDeaths")]
    // new_deaths: Option<u32>,
    // deaths: Option<u32>,
    // #[serde(rename = "newCases")]
    // news_cases: Option<u32>,
    // #[serde(rename = "totalCasesMS")]
    // total_cases: Option<u32>,
    // #[serde(rename = "deathsMS")]
    // deaths_ms: Option<u32>,
    // #[serde(rename = "totalCasesMS")]
    // total_cases_ms: Option<u32>,
    // deaths_per_100k_inhabitants: Option<f64>,
    // #[serde(rename = "totalCases_per_100k_inhabitants")]
    // total_cases_per_100k_inhabitants: Option<f64>,
    // #[serde(rename = "deaths_by_totalCases")]
    // deaths_by_total_cases: Option<f64>,
    // recovered: Option<u32>,
    // suspects: Option<u32>,
    // tests: Option<u32>,
    // tests_per_100k_inhabitants: Option<f64>,
    vaccinated: Option<u32>,
    vaccinated_per_100_inhabitants: Option<f64>,
    vaccinated_second: Option<u32>,
    vaccinated_second_per_100_inhabitants: Option<f64>,
    vaccinated_single: Option<u32>,
    vaccinated_single_per_100_inhabitants: Option<f64>,
}

impl RecordCovid19br {
    fn vaccinated_total(&self) -> u32 {
        self.vaccinated.unwrap_or_default()
            + self.vaccinated_second.unwrap_or_default()
            + (self.vaccinated_single.unwrap_or_default() * 2)
    }
}

/// Get last vacciation data from covid19br for the given country using
/// the previously downloaded CSV.
fn get_last_vaccination_data_covid19br(
    csv_text: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(RecordCovid19br, u32, u32)> {
    let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
    let mut v: Vec<RecordCovid19br> = Vec::new();
    let br_now = now.with_timezone(&chrono::FixedOffset::west(3 * 3600));
    let current_date_str = format!("{}", br_now.format("%Y-%m-%d"));
    log::debug!(
        "Ignoring possibly incomplete data from current date: {}",
        current_date_str
    );
    for result in rdr.deserialize() {
        let record: RecordCovid19br = match result {
            Ok(r) => r,
            Err(e) => {
                println!("Error parsing record: {:?}", e);
                continue;
            }
        };
        if record.state == "TOTAL" && record.date != current_date_str {
            v.push(record);
        }
    }
    let first_record = &v[v.len() - 8];
    let last_record = v.last().unwrap();
    let total_last7_vaccinations = last_record.vaccinated_total() - first_record.vaccinated_total();
    let daily_vaccinations = total_last7_vaccinations / 7;
    log::debug!("last_record = {:?}", last_record);
    log::debug!(
        "{}: total vaccinations = {}",
        first_record.date,
        first_record.vaccinated_total()
    );
    log::debug!(
        "{}: total vaccinations = {}",
        last_record.date,
        last_record.vaccinated_total()
    );
    Ok((
        last_record.clone(),
        last_record.vaccinated_total(),
        daily_vaccinations,
    ))
}

/// Get immunization estimate for Brazil given the total and daily vaccination statistics.
fn get_brazil_immunization_estimate(
    total_vaccinations: u32,
    daily_vaccinations: u32,
) -> chrono::Duration {
    // https://ftp.ibge.gov.br/Estimativas_de_Populacao/Estimativas_2020/POP2020_20210204.pdf
    let herd_size = (BRAZIL_POPULATION * 7) / 10;
    let doses = std::cmp::max(herd_size * 2 - total_vaccinations, 0);
    let days = doses / daily_vaccinations;
    log::debug!("BRAZIL_POPULATION  = {}; herd_size = {}; total_vaccinations = {}, doses = {}; daily_vaccinations = {}, days = {}",
        BRAZIL_POPULATION, herd_size, total_vaccinations, doses, daily_vaccinations, days);
    chrono::Duration::days(days.into())
}

/// Format an estimate in portuguese ("faltam X anos, Y meses e Z dias")
fn format_estimate(now: chrono::DateTime<chrono::Utc>, estimate: chrono::Duration) -> String {
    let end = now + estimate;
    log::debug!("Calculating difference between {} and {}", &now, &end);
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
        0 => "0 dias".to_string(),
        1 => vec[0].clone(),
        2 => vec.as_slice().join(" e "),
        3 => format!("{}, {} e {}", vec[0], vec[1], vec[2]),
        _ => unreachable!(),
    };
    if vec.len() == 1 && &vec[0][0..1] == "1" {
        return format!("falta {}", s);
    }
    return format!("faltam {}", s);
}

/// Format the full estimate text that will be tweeted and included in the HTML.
fn format_full_estimate(now: chrono::DateTime<chrono::Utc>, estimate: chrono::Duration) -> String {
    if estimate.num_days() == 0 {
        return "O Brasil está finalmente imunizado!".to_string();
    }
    let s = format_estimate(now, estimate);
    return format!(
        "No ritmo atual de vacinação, {} para o Brasil se imunizar contra o novo coronavírus.",
        s
    );
}

fn format_progress(data: &RecordCovid19br) -> Result<String> {
    let progress_single = data
        .vaccinated_single_per_100_inhabitants
        .ok_or_else(|| anyhow!("missing vaccination data"))?
        / 100.0;
    let progress1 = data
        .vaccinated_per_100_inhabitants
        .ok_or_else(|| anyhow!("missing vaccination data"))?
        / 100.0
        + progress_single;
    let progress2 = data
        .vaccinated_second_per_100_inhabitants
        .ok_or_else(|| anyhow!("missing vaccination data"))?
        / 100.0
        + progress_single;
    let n1 = std::cmp::min((progress1 * 20.0) as usize, 20);
    let n2 = std::cmp::min((progress2 * 20.0) as usize, 20);

    Ok(format!(
        "1ª dose:\n{}{} {:.01}%\n\n2ª dose:\n{}{} {:.01}%",
        "▓".repeat(n1 as usize),
        "░".repeat(20 - n1),
        progress1 * 100.0,
        "▓".repeat(n2 as usize),
        "░".repeat(20 - n2),
        progress2 * 100.0
    )
    .replace(".", ","))
}

/// Main function. Process arguments.
// TODO: use `clap`? Seems overkill.
#[tokio::main]
async fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let mut print = false;
    if args.len() >= 2 {
        match args[1].as_str() {
            "login" => {
                log_in().await;
                return;
            }
            "-n" => {
                print = true;
            }
            _ => {}
        }
    }
    post_estimate(print).await.unwrap();
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
    fn get_last_vaccination_data_covid19br_works() {
        let test_csv = include_str!("./testdata/cases-brazil-states.csv");
        let now = chrono::Utc.ymd(2021, 4, 24).and_hms(0, 0, 0);
        let d = get_last_vaccination_data_covid19br(&test_csv, now).unwrap();
        println!("{:?} {} {}", d.0, d.1, d.2);
        assert_eq!(d.1, 39220391);
        assert_eq!(d.2, 738580);
        // assert_eq!(
        //     d.0.vaccinated_second_per_100k
        //         .expect("should have vaccination data"),
        //     5365.29663
        // );
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
        let r = format_estimate(start, chrono::Duration::days(0));
        assert_eq!(r, "faltam 0 dias");
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

    #[test]
    fn format_tweet_works() {
        let start = chrono::Utc.ymd(2021, 3, 16).and_hms(0, 0, 0);
        let r = format_full_estimate(start, chrono::Duration::days(1));
        assert_eq!(
            r,
            "No ritmo atual de vacinação, falta 1 dia para o Brasil se imunizar contra o novo coronavírus."
        );
    }

    #[test]
    fn test_date() {
        let now = chrono::Utc.ymd(2021, 3, 31).and_hms(15, 0, 0);
        let estimate = chrono::Duration::days(646);
        let end = now + estimate;
        let _components = date_component::calculate(&now, &end);
    }
}
