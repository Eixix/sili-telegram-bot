use crate::{config::Account, storage::html};
use anyhow::Result;
use chrono::{Datelike, NaiveDate};

pub fn next_birthday(date: NaiveDate, today: NaiveDate) -> NaiveDate {
    // Celebrate February 29 on February 28 in non-leap years.
    let occurrence = |year| {
        NaiveDate::from_ymd_opt(year, date.month(), date.day())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 2, 28).unwrap())
    };
    let this_year = occurrence(today.year());
    if this_year < today {
        occurrence(today.year() + 1)
    } else {
        this_year
    }
}
pub fn messages(
    accounts: &[Account],
    today: NaiveDate,
) -> Result<(String, Option<String>, Option<String>)> {
    let mut all = "Geburtstage:".to_owned();
    let mut upcoming = String::new();
    let mut now = String::new();
    for account in accounts {
        let Some(date) = &account.birthday else {
            continue;
        };
        let date = next_birthday(NaiveDate::parse_from_str(date, "%d.%m.%Y")?, today);
        let name = html(&account.name);
        all.push_str(&format!(
            "\n{name}: {}{}",
            date.format("%d.%m.%Y"),
            if date == today {
                " (Happy Birthday!)"
            } else {
                ""
            }
        ));
        if (date - today).num_days() == 10 {
            if upcoming.is_empty() {
                upcoming.push_str("Meine Damen und Herren der Sili-Gemeinde, darf ich Sie höflich darauf hinweisen, dass in der nächsten Zeit ein Geburtstag ansteht. Bitte beraten Sie sich zeitnah, um ein adäquates Geschenk bereit zu haben.");
            }
            upcoming.push_str(&format!(
                "\n\nGeburtstagskind: {name} - {}",
                date.format("%d.%m.%Y")
            ));
        }
        if date == today {
            now.push_str(&format!("Alles Gute zu deinem Geburtstag {name}. Du bist geil!\n\n@Rest: Jetzt noch schnell das Geschenk (Dota+) kaufen! Hopp Hopp!\n"));
        }
    }
    let upcoming =
        (!upcoming.is_empty()).then(|| format!("{upcoming}\n\nIhr freundlicher Sili-Bot"));
    let now = (!now.is_empty()).then(|| format!("{now}\n\nDein geliebter Sili-Bot"));
    Ok((all, upcoming, now))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_and_year_boundary() {
        let leap = NaiveDate::from_ymd_opt(2000, 2, 29).unwrap();
        assert_eq!(
            next_birthday(leap, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).to_string(),
            "2026-02-28"
        );
        assert_eq!(
            next_birthday(leap, NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()).to_string(),
            "2028-02-29"
        );
        let accounts = [Account {
            identifier: "1".into(),
            name: "Test".into(),
            birthday: Some("1.1.2000".into()),
        }];
        assert!(
            messages(&accounts, NaiveDate::from_ymd_opt(2026, 12, 22).unwrap())
                .unwrap()
                .1
                .is_some()
        );
    }
}
