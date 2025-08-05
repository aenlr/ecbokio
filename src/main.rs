use crate::bokio::{Bokio, CreateJournal, CreateJournalAccount, JournalEntry, BOKIO_API_URL};
use crate::easycashier::{DateRequest, EasyCashier, ZRapport, EASYCASHIER_URL};
use chrono::naive::NaiveDate;
use chrono::Days;
use rust_decimal::Decimal;
use std::io::{IsTerminal, Write};
use std::str::FromStr;
use ureq::Error;
use utils::PageReq;

mod bokio;
mod easycashier;
mod utils;

struct Cli {
    orgnummer: String,
    easycashier_url: String,
    easycashier_username: String,
    easycashier_password: String,
    bokio_api_url: String,
    bokio_api_token: String,
    bokio_company_id: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

fn check_arg(name: &str, arg: &str, iter: &mut impl Iterator<Item = String>) -> Option<String> {
    let prefix = "--".to_string() + name;
    if arg == prefix {
        let val = iter.next();
        return Some(val.expect(&format!("{} expected value", prefix)));
    }

    let prefix = prefix + "=";
    if let Some(val) = arg.strip_prefix(&prefix) {
        let val = Some(val)
            .filter(|s| !s.is_empty())
            .expect(&format!("{} expected value", prefix));
        return Some(val.to_string());
    }

    None
}

fn check_args(
    names: &[&str],
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Option<String> {
    names.iter().find_map(|name| check_arg(name, arg, iter))
}

struct RapportImport {
    rapport: ZRapport,
    verifikat: Option<JournalEntry>,
}

fn hamta_rapporter(
    args: &Cli,
    easy: &EasyCashier,
    bokio: &Bokio,
) -> Result<(Vec<RapportImport>, DateRequest), Error> {
    let mut page = PageReq { page: 1, size: 100 };
    let date_req = DateRequest::new(&args.start_date, &args.end_date);
    let bokio_start_date = date_req.start_date.checked_sub_days(Days::new(14));
    let journal = bokio
        .list_journal(bokio_start_date, Some(date_req.end_date))?;
    let mut importer: Vec<RapportImport> = Vec::new();
    loop {
        let rapporter = easy.zrapporter(&date_req, &page)?;
        if rapporter.items.is_empty() {
            break;
        }

        for rapport in rapporter.items {
            let title = rapport.verifikatnamn();
            let verifikat = journal
                .iter()
                .find(|e| e.title == title && e.reversed_by_journal_entry_id.is_none());
            importer.push(RapportImport {
                rapport,
                verifikat: verifikat.cloned(),
            })
        }

        if page.page >= rapporter.meta_information.total_pages {
            break;
        }
        page.page += 1;
    }

    Ok((importer, date_req))
}

fn rakna_importerade_rapporter(importer: &Vec<RapportImport>) -> usize {
    importer.iter().filter(|e| e.verifikat.is_some()).count()
}

fn lista_rapporter(importer: &Vec<RapportImport>) {
    println!(
        "| ✅ |   NR | DATUM      | {:<39} |     KORT |  KONTANT |   SWISH | VERNR |",
        "TITEL"
    );
    println!(
        "|---|------|------------|-{:-<39}-|----------|----------|---------|-------|",
        "-"
    );
    for e in importer {
        let rapport = &e.rapport;
        let verifikat = &e.verifikat;
        let title = rapport.verifikatnamn();
        let datum = rapport.datum();
        let kort = rapport.konto(1580);
        let kontant = rapport.konto(1911);
        let swish = rapport.konto(1932);
        let vernr = verifikat
            .clone()
            .map_or("".to_string(), |j| j.journal_entry_number);
        let marker = if e.verifikat.is_some() { "✅" } else { " " };
        println!(
            "| {} | {:4} | {} | {:<39} | {:8.2} | {:8.2} | {:7.2} | {:<5} |",
            marker, rapport.sequence_number, datum, title, kort, kontant, swish, vernr
        );
    }
}

fn valj_rapporter(rapporter: &Vec<RapportImport>) -> Vec<u32> {
    let mojliga = rapporter
        .iter()
        .filter(|e| e.verifikat.is_none())
        .map(|e| e.rapport.sequence_number)
        .collect::<Vec<_>>();

    if mojliga.is_empty() {
        return Vec::new();
    }

    loop {
        if mojliga.len() == 1 {
            print!(
                "Importera rapport {} ([J]a, [N]ej)? ",
                mojliga.first().unwrap()
            );
        } else {
            print!("Importera ([J]a = alla, [N]ej = ingen eller nummer)? ");
        }

        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if let Ok(size) = std::io::stdin().read_line(&mut input) {
            if size == 0 {
                // EOF
                return Vec::new();
            }

            if input == "\n" {
                return mojliga;
            }

            input = input.trim().to_lowercase();
            if input.is_empty() {
                continue;
            }

            if input == "j" || input == "y" {
                return mojliga;
            }

            if input == "n" || input == "q" {
                return Vec::new();
            }

            let mut valda: Vec<u32> = Vec::new();
            for part in input.split_whitespace() {
                if let Ok(n) = part.parse::<u32>()
                    && mojliga.contains(&n)
                {
                    valda.push(n);
                } else {
                    println!("Ogiltigt val: {}", part);
                    valda.clear();
                    break;
                }
            }

            if !valda.is_empty() {
                return valda;
            }
        }
    }
}

fn create_journal_entry(rapport: &ZRapport) -> CreateJournal {
    let title = rapport.verifikatnamn();
    let date = rapport.datum();
    let mut items: Vec<CreateJournalAccount> =
        Vec::with_capacity(rapport.z_report_transactions.len());
    let zero = Decimal::from(0);
    for tr in rapport.z_report_transactions.iter() {
        let debit = tr.amount.max(zero);
        let credit = tr.amount.min(zero).abs();
        let account = tr.account_number as i32;
        items.push(CreateJournalAccount {
            account,
            debit,
            credit,
        })
    }

    CreateJournal { title, date, items }
}

fn importera_rapport(
    easy: &EasyCashier,
    bokio: &Bokio,
    import: &RapportImport,
) -> Result<JournalEntry, String> {
    println!(
        "Importerar Z-Rapport {} ...",
        import.rapport.sequence_number
    );

    print!("* Hämtar PDF... ");
    std::io::stdout().flush().ok();
    let (pdf, pdf_filename) = easy.zrapport_pdf(&import.rapport).map_err(|e| {
        format!(
            "Kunde inte hämta PDF för Z-Rapport {}: {}",
            import.rapport.sequence_number, e
        )
    })?;
    println!("{}", pdf_filename);
    std::fs::write(&pdf_filename, pdf).expect("Kunde inte spara PDF.");
    let json_filename = pdf_filename.replace(".pdf", ".json");
    let json = serde_json::to_vec_pretty(&import.rapport).unwrap();
    print!("* Sparar {}...", json_filename);
    std::io::stdout().flush().ok();
    std::fs::write(&json_filename, json).expect("Kunde inte spara JSON.");

    let journal_entry = create_journal_entry(&import.rapport);
    let json_filename = pdf_filename.replace(".pdf", "_bokio.json");
    print!(" {}...", json_filename);
    std::io::stdout().flush().ok();
    let json = serde_json::to_vec_pretty(&journal_entry).unwrap();
    std::fs::write(&json_filename, json).expect("Kunde inte spara JSON.");
    println!();

    print!("* Bokför Z-Rapport {}... ", import.rapport.sequence_number);
    std::io::stdout().flush().ok();
    let journal_entry = bokio.create_journal_entry(&journal_entry).map_err(|e| {
        format!(
            "Kunde inte bokföra verifikat för Z-Rapport {}: {}",
            import.rapport.sequence_number, e
        )
    })?;
    println!("{}", journal_entry.journal_entry_number);

    println!("* Laddar upp underlag");
    bokio
        .upload(&pdf_filename, "application/pdf", &journal_entry.id)
        .inspect_err(|e| {
            eprintln!(
                "Kunde inte ladda upp PDF för Z-Rapport {} till verifikat {}: {}",
                import.rapport.sequence_number, journal_entry.journal_entry_number, e
            )
        })
        .ok();

    Ok(journal_entry)
}

fn importera(easy: &EasyCashier, bokio: &Bokio, rapporter: &mut Vec<RapportImport>) {
    loop {
        lista_rapporter(&rapporter);
        let valda = valj_rapporter(&rapporter);
        if valda.is_empty() {
            break;
        }

        for seqnr in valda {
            let imp = rapporter
                .iter_mut()
                .find(|e| e.rapport.sequence_number == seqnr)
                .unwrap();
            match importera_rapport(&easy, &bokio, imp) {
                Ok(journal_entry) => {
                    imp.verifikat.replace(journal_entry);
                }
                Err(msg) => {
                    eprintln!("{}", msg);
                    break;
                }
            }
        }
    }
}

fn read_prompt(prompt: &str) -> std::io::Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush().and_then(|_| {
        let mut val = String::new();
        match std::io::stdin().read_line(&mut val) {
            Ok(_) => Ok(val.trim().to_string()),
            Err(e) => Err(e),
        }
    })
}

fn read_prompt_trim(prompt: &str) -> String {
    read_prompt(prompt).unwrap().trim().to_string()
}

fn read_password(prompt: &str) -> std::io::Result<String> {
    // IntelliJ console is broken giving "device not ready" for /dev/tty.
    // Strangely the builtin terminal works fine.
    if std::io::stdin().is_terminal() && !std::env::var("BROKEN_TERMINAL").is_ok() {
        rpassword::prompt_password(prompt)
    } else {
        read_prompt(prompt)
    }
}

fn read_password_trim(prompt: &str) -> String {
    read_password(prompt).unwrap().trim().to_string()
}

fn main() {
    let mut args = Cli {
        orgnummer: std::env::var("EASYCASHIER_COMPANY").unwrap_or("".to_string()),
        start_date: None,
        end_date: None,
        easycashier_url: std::env::var("EASYCASHIER_URL").unwrap_or(EASYCASHIER_URL.to_string()),
        easycashier_username: std::env::var("EASYCASHIER_USERNAME").unwrap_or("".to_string()),
        easycashier_password: std::env::var("EASYCASHIER_PASSWORD").unwrap_or("".to_string()),
        bokio_api_url: std::env::var("BOKIO_API_URL").unwrap_or(BOKIO_API_URL.to_string()),
        bokio_api_token: std::env::var("BOKIO_API_TOKEN").unwrap_or("".to_string()),
        bokio_company_id: std::env::var("BOKIO_COMPANY_ID").unwrap_or("".to_string()),
    };

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        if let Some(url) = check_args(&["easycashier-url", "easy-url"], &arg, &mut iter) {
            args.easycashier_url = url;
        } else if let Some(username) =
            check_args(&["easycashier-username", "easy-username"], &arg, &mut iter)
        {
            args.easycashier_username = username;
        } else if let Some(password) =
            check_args(&["easycashier-password", "easy-password"], &arg, &mut iter)
        {
            args.easycashier_password = password;
        } else if let Some(orgnummer) = check_args(
            &["orgnummer", "easycashier-company", "easy-company"],
            &arg,
            &mut iter,
        ) {
            args.orgnummer = orgnummer;
        } else if let Some(start) = check_arg("date", &arg, &mut iter)
            .map(|v| NaiveDate::from_str(&v).unwrap())
        {
            args.start_date = Some(start);
            args.end_date = Some(start);
        } else if let Some(start) = check_args(&["start-date", "start"], &arg, &mut iter)
            .map(|v| NaiveDate::from_str(&v).unwrap())
        {
            args.start_date = Some(start);
        } else if let Some(end) = check_args(&["end-date", "end"], &arg, &mut iter)
            .map(|v| NaiveDate::from_str(&v).unwrap())
        {
            args.end_date = Some(end);
        } else if let Some(url) = check_args(&["bokio-api-url", "bokio-url"], &arg, &mut iter) {
            args.bokio_api_url = url;
        } else if let Some(token) = check_args(&["bokio-api-token", "bokio-token"], &arg, &mut iter)
        {
            args.bokio_api_token = token;
        } else if let Some(company_id) =
            check_args(&["bokio-company-id", "bokio-company"], &arg, &mut iter)
        {
            args.bokio_company_id = company_id;
        } else {
            eprintln!("{}: invalid option", arg);
            std::process::exit(1);
        }
    }

    if !args.orgnummer.is_empty() {
        let mut orgnr = args.orgnummer.trim().to_string();
        let len = orgnr.len();
        if orgnr.chars().all(|c| c.is_ascii_digit()) && [10, 12].contains(&len) {
            orgnr = format!("{}-{}", &orgnr[0..len - 4], &orgnr[len - 4..len]);
        }
        args.orgnummer = orgnr;
    }

    if args.easycashier_username.is_empty() {
        let username = read_prompt_trim("EasyCashier username: ");
        if username.is_empty() {
            return;
        }
        args.easycashier_username = username;
    }

    if args.easycashier_password.is_empty() {
        let password = read_password_trim("EasyCashier password: ");
        if password.is_empty() {
            return;
        }
        args.easycashier_password = password;
    }

    if args.bokio_api_token.is_empty() {
        let token = read_password_trim("Bokio API token: ");
        if token.is_empty() {
            return;
        }
        args.bokio_api_token = token;
    }

    if args.bokio_company_id.is_empty() {
        let company_id = read_prompt_trim("Bokio company id: ");
        if company_id.is_empty() {
            return;
        }
        args.bokio_company_id = company_id;
    }

    let easy = EasyCashier::login(
        &args.easycashier_url,
        &args.easycashier_username,
        &args.easycashier_password,
        &args.orgnummer,
    );
    let easy = easy
        .inspect_err(|err| {
            eprintln!("EasyCashier: inloggning misslyckades: {}", err);
            std::process::exit(1);
        })
        .unwrap();

    let bokio = Bokio::new(
        &args.bokio_api_url,
        &args.bokio_company_id,
        &args.bokio_api_token,
    );

    let (mut rapporter, dates) = hamta_rapporter(&args, &easy, &bokio)
        .inspect_err(|err| {
            eprintln!("Kunde inte hämta Z-Rapporter: {}", err);
            std::process::exit(1);
        })
        .unwrap();

    println!(
        "{} Z-Rapporter för {} ({} - {})",
        rapporter.len(),
        easy.company,
        utils::format_local_date(&dates.start_date),
        utils::format_local_date(&dates.end_date),
    );

    if !rapporter.is_empty() {
        let antal_skippade = rakna_importerade_rapporter(&rapporter);
        importera(&easy, &bokio, &mut rapporter);
        let antal_importerade = rakna_importerade_rapporter(&rapporter) - antal_skippade;

        println!();
        println!("{} Z-Rapporter importerades", antal_importerade);
        if antal_skippade > 0 {
            println!("{} Z-Rapporter redan importerade", antal_skippade);
        }
    }
}
