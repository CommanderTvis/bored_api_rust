#![allow(unused)]

mod boredapi {
    use std::str::FromStr;
    use strum_macros;
    use std::{fmt, collections, marker};
    use std::borrow::Borrow;
    use std::cmp;

    #[derive(strum_macros::EnumString, strum_macros::ToString, cmp::PartialEq)]
    pub enum ActivityType {
        #[strum(serialize = "education")]
        Education,
        #[strum(serialize = "recreational")]
        Recreational,
        #[strum(serialize = "social")]
        Social,
        #[strum(serialize = "diy")]
        Diy,
        #[strum(serialize = "charity")]
        Charity,
        #[strum(serialize = "cooking")]
        Cooking,
        #[strum(serialize = "relaxation")]
        Relaxation,
        #[strum(serialize = "music")]
        Music,
        #[strum(serialize = "busywork")]
        Busywork,
    }

    pub enum Error {
        HttpError(reqwest::Error),
        ApiError(String),
        BadResponse(i64),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            return match self {
                Error::HttpError(e) => write!(f, "HttpError({})", e.to_string()),
                Error::ApiError(e) => write!(f, "ApiError({})", e),
                Error::BadResponse(i) => write!(f, "BadResponse({})", i),
            };
        }
    }

    pub struct Activity {
        pub description: String,
        pub accessibility: f64,
        pub activity_type: ActivityType,
        pub participants: u64,
        pub price: f64,
        pub link: Option<String>,
        pub key: u64,
    }

    impl fmt::Display for Activity {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f,
                   "Activity(description={}, accessibility={}, activity_type={}, participants={}, price={}, link={}, key={})",
                   self.description,
                   self.accessibility,
                   self.activity_type.to_string(),
                   self.participants,
                   self.price,
                   self.link.as_ref().unwrap_or(&"None".to_string()),
                   self.key)
        }
    }

    pub struct ActivityCriterion<T> {
        name: &'static str,
        phantom: marker::PhantomData<T>,
    }

    pub const EXACT_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "accessibility",
        phantom: marker::PhantomData,
    };

    pub const EXACT_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "price",
        phantom: marker::PhantomData,
    };

    pub const KEY: ActivityCriterion<u64> = ActivityCriterion {
        name: "key",
        phantom: marker::PhantomData,
    };

    pub const MAX_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "maxaccessibility",
        phantom: marker::PhantomData,
    };

    pub const MAX_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "maxprice",
        phantom: marker::PhantomData,
    };

    pub const MIN_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "minaccessibility",
        phantom: marker::PhantomData,
    };

    pub const MIN_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "minprice",
        phantom: marker::PhantomData,
    };

    pub const PARTICIPANTS: ActivityCriterion<u64> = ActivityCriterion {
        name: "participants",
        phantom: marker::PhantomData,
    };

    pub const TYPE: ActivityCriterion<ActivityType> = ActivityCriterion {
        name: "type",
        phantom: marker::PhantomData,
    };

    pub struct CriteriaSelection { parameters: collections::HashMap<String, String> }

    impl CriteriaSelection {
        pub fn set<T: ToString>(mut self, criterion: ActivityCriterion<T>, value: T) -> Self {
            self.parameters.insert(criterion.name.to_string(), value.to_string());
            return self;
        }
    }

    impl Clone for CriteriaSelection {
        fn clone(&self) -> Self {
            CriteriaSelection { parameters: self.parameters.clone() }
        }
    }

    impl Default for CriteriaSelection {
        fn default() -> Self {
            CriteriaSelection { parameters: collections::HashMap::new() }
        }
    }

    pub struct BoredApi {
        pub url: &'static str,
        pub client: reqwest::Client,
    }

    impl Default for BoredApi {
        fn default() -> Self {
            BoredApi { url: "http://www.boredapi.com/api/activity", client: reqwest::Client::new() }
        }
    }

    impl BoredApi {
        pub async fn random(self) -> Result<Activity, Error> {
            self.request_activity(&collections::HashMap::new()).await
        }

        pub async fn by_criteria<F: FnOnce(CriteriaSelection) -> CriteriaSelection>(self, selection: F) -> Result<Activity, Error> {
            let mut sel = CriteriaSelection::default();
            sel = selection(sel);
            return self.request_activity(sel.parameters.borrow()).await;
        }

        async fn request_activity(self, params: &collections::HashMap<String, String>) -> Result<Activity, Error> {
            return match self.client.get(self.url).query(&params).send().await {
                Ok(r) => match r.json::<serde_json::Value>().await {
                    Ok(val) => self.deserialize(val),
                    Err(r) => Err(Error::HttpError(r))
                },
                Err(r) => Err(Error::HttpError(r)),
            };
        }

        fn deserialize(self, json: serde_json::Value) -> Result<Activity, Error> {
            if let Some(err) = json.get("errors") {
                return Err(err
                    .as_str()
                    .map(|s| Error::ApiError(s.to_string()))
                    .unwrap_or(Error::BadResponse(0)));
            }

            return Ok(Activity {
                description: json.get("activity").ok_or(Error::BadResponse(1))?.as_str()
                    .ok_or(Error::BadResponse(2))?
                    .to_string(),

                accessibility: json.get("accessibility").ok_or(Error::BadResponse(3))?.as_f64()
                    .ok_or(Error::BadResponse(4))?,

                activity_type: ActivityType::from_str(json.get("type").ok_or(Error::BadResponse(5))?
                    .as_str()
                    .ok_or(Error::BadResponse(6))?)
                    .unwrap(),

                participants: json.get("participants").ok_or(Error::BadResponse(7))?.as_u64()
                    .ok_or(Error::BadResponse(8))?,

                price: json.get("price").ok_or(Error::BadResponse(9))?.as_f64()
                    .ok_or(Error::BadResponse(10))?,

                link: match json.get("link").map(|s| s.as_str()) {
                    None => None,
                    Some(some) => Some(some.ok_or(Error::BadResponse(11))?.to_string()),
                },

                key: json.get("key").ok_or(Error::BadResponse(12))?.as_str()
                    .ok_or(Error::BadResponse(13))
                    ?.parse::<u64>()
                    .map_err(|e| Error::BadResponse(14))?,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::boredapi;
    use tokio::runtime::Runtime;
    use crate::boredapi::{Error, Activity};

    macro_rules! aw {
    ($e:expr) => {
        Runtime::new().expect("").block_on($e)
    };
  }

    #[test]
    fn random() {
        match aw!(boredapi::BoredApi::default().random()) {
            Ok(a) => { println!("{}", a); }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn by_criteria() {
        match aw!(boredapi::BoredApi::default().by_criteria(|sel| sel.set(boredapi::TYPE, boredapi::ActivityType::Busywork))) {
            Ok(a) => {
                assert!(a.activity_type == boredapi::ActivityType::Busywork);
                println!("{}", a)
            }
            Err(_) => assert!(false),
        }
    }
}
