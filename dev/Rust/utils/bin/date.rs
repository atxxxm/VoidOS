use time::{format_description, OffsetDateTime};

fn main() {
    let now = OffsetDateTime::now_utc();
    let format =
        format_description::parse_borrowed::<2>("[year]-[month]-[day] [hour]:[minute]:[second] UTC")
            .expect("static format description is valid");

    match now.format(&format) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("date: {e}"),
    }
}
