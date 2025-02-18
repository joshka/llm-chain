//! Parameters are used to pass data steps of the chain. This module implements them.
//!
//! Parameters are used to pass data between steps of the chain. They are used to fill in the prompt template, and are also filled in by the output of the previous step. Parameters have a special key, `text`, which is used as a default key for simple use cases.
use crate::output::Output;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

type Map = BTreeMap<String, Box<dyn ParamFull>>;

/// Parameters define the parameters sent into each step. The parameters are used to fill in the prompt template, and are also filled in by the output of the previous step. Parameters have a special key, `text`, which is used as a default key for simple use cases.
///
/// Parameters also implement a few convenience conversion traits to make it easier to work with them.
///
/// # Examples
///
/// **Creating a default parameter from a string**
/// ```
/// use llm_chain::Parameters;
/// let p: Parameters = "Hello world!".into();
/// assert_eq!(p.get("text").unwrap().as_str(), "Hello world!");
/// ```
/// **Creating a list of parameters from a list of pairs**
/// ```
/// use llm_chain::Parameters;
/// let p: Parameters = vec![("text", "Hello world!"), ("name", "John Doe")].into();
/// assert_eq!(p.get("text").unwrap().as_str(), "Hello world!");
/// assert_eq!(p.get("name").unwrap().as_str(), "John Doe");
/// ```
#[derive(Default, Debug)]
pub struct Parameters {
    map: Map,
}

impl Clone for Parameters {
    fn clone(&self) -> Self {
        let mut map = Map::new();
        for (key, value) in self.map.iter() {
            map.insert(key.clone(), value.boxed_clone());
        }
        Self { map }
    }
}

impl PartialEq for Parameters {
    fn eq(&self, other: &Self) -> bool {
        self.map.keys().len() == other.map.keys().len()
            && self.map.iter().all(|(k, v)| {
                if let Some(other_v) = other.map.get(k) {
                    v.get() == other_v.get()
                } else {
                    false
                }
            })
    }
}

pub trait Param: Send + Sync {
    fn get(&self) -> String;
}

/// This trait is used to implement a dynamic parameter this shouldn't be used but exists only for internal purposes.
#[doc(hidden)]
pub trait ParamFull: Param + Debug + Send + Sync {
    #[doc(hidden)]
    fn boxed_clone(&self) -> Box<dyn ParamFull + Send>;
}

impl<T: Param + Debug + Clone + 'static> ParamFull for T {
    #[doc(hidden)]
    fn boxed_clone(&self) -> Box<dyn ParamFull + Send> {
        Box::new(self.clone())
    }
}
#[derive(Debug, Clone)]
struct StringParam {
    value: String,
}

impl StringParam {
    fn new(value: String) -> Self {
        Self { value }
    }
}

impl Param for StringParam {
    fn get(&self) -> String {
        self.value.clone()
    }
}

const TEXT_KEY: &str = "text";

impl Parameters {
    /// Creates a new empty set of parameters.
    pub fn new() -> Parameters {
        Default::default()
    }
    /// Creates a new set of parameters with a single key, `text`, set to the given value.
    pub fn new_with_text<T: Into<String>>(text: T) -> Parameters {
        let mut map = Map::new();
        map.insert(
            TEXT_KEY.to_string(),
            Box::new(StringParam::new(text.into())),
        );
        Parameters { map }
    }
    /// Copies the parameters and adds a new key-value pair.
    pub fn with<K: Into<String>, V: Into<String>>(&self, key: K, value: V) -> Parameters {
        let mut copy = self.clone();
        copy.map
            .insert(key.into(), Box::new(StringParam::new(value.into())));
        copy
    }

    /// Copies the parameters and adds a new key-value pair pair, where the value is a dynamic parameter.
    pub fn with_dynamic<K: Into<String>, V: ParamFull>(&self, key: K, value: V) -> Parameters {
        let mut copy = self.clone();
        copy.map.insert(key.into(), value.boxed_clone());
        copy
    }

    /// Copies the parameters and adds a new key-value pair with the key `text`, which is the default key.
    pub fn with_text<K: Into<String>>(&self, text: K) -> Parameters {
        self.with(TEXT_KEY, text)
    }
    pub async fn with_text_from_output<O: Output>(&self, output: &O) -> Parameters {
        output
            .primary_textual_output()
            .await
            .map_or(self.clone(), |text| self.with_text(text))
    }
    /// Combines two sets of parameters, returning a new set of parameters with all the keys from both sets.
    pub fn combine(&self, other: &Parameters) -> Parameters {
        let mut copy = self.clone();
        for (key, value) in other.map.iter() {
            copy.map.insert(key.clone(), value.boxed_clone());
        }
        copy
    }
    /// Returns the value of the given key, or `None` if the key does not exist.
    pub fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).map(|param| param.get())
    }

    pub fn get_text(&self) -> Option<String> {
        self.get(TEXT_KEY)
    }

    pub(crate) fn to_tera(&self) -> tera::Context {
        let mut context = tera::Context::new();
        for (key, value) in self.map.iter() {
            context.insert(key, &value.get());
        }
        context
    }

    fn from_seq<K, V, M>(m: M) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        M: IntoIterator<Item = (K, V)>,
    {
        let mut map = Map::new();
        for (k, v) in m.into_iter() {
            map.insert(k.into(), Box::new(StringParam::new(v.into())));
        }
        Parameters { map }
    }
}

impl From<String> for Parameters {
    fn from(text: String) -> Self {
        Parameters::new_with_text(text)
    }
}

impl From<&str> for Parameters {
    fn from(text: &str) -> Self {
        Parameters::new_with_text(text)
    }
}

impl From<HashMap<String, String>> for Parameters {
    fn from(map: HashMap<String, String>) -> Self {
        Parameters::from_seq(map.into_iter())
    }
}

impl From<BTreeMap<String, String>> for Parameters {
    fn from(map: BTreeMap<String, String>) -> Self {
        Parameters::from_seq(map.into_iter())
    }
}

impl From<Vec<(String, String)>> for Parameters {
    fn from(data: Vec<(String, String)>) -> Self {
        Parameters::from_seq(data.into_iter())
    }
}

impl From<Vec<(&str, &str)>> for Parameters {
    fn from(data: Vec<(&str, &str)>) -> Self {
        Parameters::from_seq(data)
    }
}

/// A macro that creates a new `Parameters` instance with the provided key-value pairs.
///
/// This macro makes it easy to create a new `Parameters` instance without having to call the constructor functions directly. It supports different input formats for creating `Parameters` instances with different key-value pairs.
///
/// # Usage
///
/// ```
/// # use llm_chain::parameters;
/// parameters!(); // Creates an empty Parameters instance.
/// ```
///
/// # Examples
///
/// ```
/// # use llm_chain::parameters;
/// // Create an empty Parameters instance.
/// let params = parameters!();
///
/// // Create a Parameters instance with the "text" key set to "some text".
/// let params_with_text = parameters!("some text");
///
/// // Create a Parameters instance with multiple key-value pairs.
/// let params_with_multiple = parameters! {
///     "key1" => "val1",
///     "key2" => "val2"
/// };
/// ```
///
/// # Parameters
///
/// - `()`: Creates an empty `Parameters` instance.
/// - `"some text"`: Creates a `Parameters` instance with the "text" key set to "some text".
/// - `{"key1" => "val1", "key2" => "val2"}`: Creates a `Parameters` instance with the specified key-value pairs.
#[macro_export]
macro_rules! parameters {
    () => {
        $crate::Parameters::new()
    };
    ($text:expr) => {
        llm_chain::Parameters::new_with_text($text)
    };
    ($($key:expr => $value:expr),+$(,)?) => {{
        let mut params = $crate::Parameters::new();
        $(
            params = params.with($key, $value);
        )+
        params
    }};
}
