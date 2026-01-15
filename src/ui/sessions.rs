use std::time::{Duration, UNIX_EPOCH};

const DATE_FORMAT_TODAY: &str = "Today %H:%M";
const DATE_FORMAT_YESTERDAY: &str = "Yesterday %H:%M";
const DATE_FORMAT_OTHER: &str = "%Y-%m-%d";

pub fn format_session_date(timestamp: u64) -> String {
    let session_time = UNIX_EPOCH + Duration::from_secs(timestamp);
    let datetime: chrono::DateTime<chrono::Local> = session_time.into();

    let today = chrono::Local::now();
    let session_date = datetime.date_naive();

    if session_date == today.date_naive() {
        datetime.format(DATE_FORMAT_TODAY).to_string()
    } else if session_date == today.date_naive() - chrono::Duration::days(1) {
        datetime.format(DATE_FORMAT_YESTERDAY).to_string()
    } else {
        datetime.format(DATE_FORMAT_OTHER).to_string()
    }
}
