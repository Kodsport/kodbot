use reqwest::{Client, Url};
use reqwest::header;

use crate::config::Config;
use crate::secrets::Secrets;

pub async fn verify_membership(email: String, config: &Config, secrets: &Secrets) -> bool {
	let client = Client::new();
	let mut url = Url::parse(&config.ebas().url()).expect("Couldn't parse URL for eBas.");
	url.path_segments_mut().expect("Couldn't get path segments for eBas URL.").push("confirm_membership.json");
	let body = serde_json::json!({
		"request" : {
			"action" : "confirm_membership",
			"association_number" : secrets.ebas.id,
			"api_key" : secrets.ebas.api_key,
			"year_id" : time::OffsetDateTime::now_utc().year(),
			"email": email,
		}
	}).to_string();
	let request = client.post(url).body(body).header(header::CONTENT_TYPE, "application/json");
	let response = request.send().await.expect("Couldn't send request to eBas.");
	let text = response.text().await.expect("Couldn't get body from response.");
	let json: serde_json::Value = serde_json::from_str(&text).expect("Couldn't parse JSON.");
	let response = &json["response"];

	if !response["request_result"]["error"].is_null() {
		// TODO Better error handling.
		return false;
	}

	if let Some(is_member) = response["member_found"].as_bool() {
		is_member
	} else {
		// TODO Again, better error handling.
		false
	}
}