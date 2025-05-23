#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

use std::borrow::Cow;

use json_patch::Patch;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::JsValue;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "actix", feature = "ssr"))] {
        mod actix;
        pub use crate::actix::*;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "axum", feature = "ssr"))] {
        mod axum;
        pub use crate::axum::*;
    }
}

/// A server signal update containing the signal type name and json patch.
///
/// This is whats sent over the SSE, and is used to patch the signal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSignalUpdate {
    name: Cow<'static, str>,
    patch: Patch,
}

impl ServerSignalUpdate {
    /// Creates a new [`ServerSignalUpdate`] from an old and new instance of `T`.
    pub fn new<T>(
        name: impl Into<Cow<'static, str>>,
        old: &T,
        new: &T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        let left = serde_json::to_value(old)?;
        let right = serde_json::to_value(new)?;
        let patch = json_patch::diff(&left, &right);
        Ok(ServerSignalUpdate {
            name: name.into(),
            patch,
        })
    }

    /// Creates a new [`ServerSignalUpdate`] from two json values.
    pub fn new_from_json<T>(name: impl Into<Cow<'static, str>>, old: &Value, new: &Value) -> Self {
        let patch = json_patch::diff(old, new);
        ServerSignalUpdate {
            name: name.into(),
            patch,
        }
    }
}

/// Provides a SSE url for server signals, if there is not already one provided.
/// This ensures that you can provide it at the highest possible level, without overwriting a SSE
/// that has already been provided (for example, by a server-rendering integration.)
///
/// Note, the server should have a route to handle this SSE.
///
/// # Example
///
/// ``` 
/// use leptos::prelude::*;
/// #[component]
/// pub fn App() -> impl IntoView {
///     // Provide SSE connection
///     leptos_sse::provide_sse("http://localhost:3000/sse").unwrap();
///
///     // ...
/// }
/// ```
#[allow(unused_variables)]
pub fn provide_sse(url: &str) -> Result<(), JsValue> {
    provide_sse_inner(url)
}

/// Creates a signal which is controlled by the server.
///
/// This signal is initialized as T::default, is read-only on the client, and is updated through json patches
/// sent through a SSE connection.
///
/// For types that are not Send + Sync, use [`create_sse_signal_local`] instead.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use serde::Deserialize;
/// use leptos::prelude::*;
/// use leptos_sse::create_sse_signal;
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// pub struct Count {
///     pub value: i32,
/// }
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     // Create server signal
///     let count = create_sse_signal::<Count>("counter");
///
///     view! {
///         <h1>"Count: " {move || count.get().value.to_string() }</h1>
///     }
/// }
/// ```
#[allow(unused_variables)]
pub fn create_sse_signal<T>(name: impl Into<Cow<'static, str>>) -> ReadSignal<T>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    let name = name.into();
    let (get, set) = signal(T::default());
    
    #[cfg(target_arch = "wasm32")]
    setup_sse_signal(name, set);

    get
}

/// Creates a signal which is controlled by the server for types that are not Send + Sync.
///
/// This is the same as [`create_sse_signal`] but uses LocalStorage for signals that don't
/// implement Send + Sync.
#[allow(unused_variables)]
pub fn create_sse_signal_local<T>(name: impl Into<Cow<'static, str>>) -> ReadSignal<T, LocalStorage>
where
    T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
{
    let name = name.into();
    let (get, set) = signal_local(T::default());
    
    #[cfg(target_arch = "wasm32")]
    setup_sse_signal_local(name, set);

    get
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::collections::HashMap;
        use std::rc::Rc;
        use std::cell::RefCell;
        use std::sync::{Arc, Mutex};

        use web_sys::EventSource;
        use leptos::prelude::*;

        // Thread-local storage for EventSource since it's not Send + Sync
        thread_local! {
            static EVENT_SOURCE: RefCell<Option<EventSource>> = RefCell::new(None);
            static STATE_SIGNALS: RefCell<HashMap<Cow<'static, str>, RwSignal<Value>>> = RefCell::new(HashMap::new());
            static STATE_SIGNALS_LOCAL: RefCell<HashMap<Cow<'static, str>, RwSignal<Value, LocalStorage>>> = RefCell::new(HashMap::new());
            static DELAYED_UPDATES: RefCell<HashMap<Cow<'static, str>, Vec<Patch>>> = RefCell::new(HashMap::new());
        }

        /// Context marker to indicate SSE has been initialized
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct SseInitialized;

        fn setup_sse_signal<T>(name: Cow<'static, str>, set: WriteSignal<T>)
        where
            T: Default + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
        {
            use leptos::prelude::*;

            let signal = RwSignal::new(serde_json::to_value(T::default()).unwrap());
            
            if use_context::<SseInitialized>().is_some() {
                leptos::logging::log!("Setting up SSE signal: {}", name);
                
                STATE_SIGNALS.with(|signals| {
                    signals.borrow_mut().insert(name.clone(), signal);
                });

                Effect::new(move |_| {
                    let new_value = serde_json::from_value(signal.get()).unwrap();
                    set.set(new_value);
                });
            } else {
                leptos::logging::error!(
                    r#"server signal was used without a SSE being provided.

Ensure you call `leptos_sse::provide_sse("http://localhost:3000/sse")` at the highest level in your app."#
                );
            }
        }

        fn setup_sse_signal_local<T>(name: Cow<'static, str>, set: WriteSignal<T, LocalStorage>)
        where
            T: Default + Serialize + for<'de> Deserialize<'de> + 'static,
        {
            use leptos::prelude::*;

            let signal = RwSignal::new_local(serde_json::to_value(T::default()).unwrap());
            
            if use_context::<SseInitialized>().is_some() {
                STATE_SIGNALS_LOCAL.with(|signals| {
                    signals.borrow_mut().insert(name.clone(), signal);
                });

                Effect::new(move |_| {
                    let new_value = serde_json::from_value(signal.get()).unwrap();
                    set.set(new_value);
                });
            } else {
                leptos::logging::error!(
                    r#"server signal was used without a SSE being provided.

Ensure you call `leptos_sse::provide_sse("http://localhost:3000/sse")` at the highest level in your app."#
                );
            }
        }

        #[inline]
        fn provide_sse_inner(url: &str) -> Result<(), JsValue> {
            use web_sys::MessageEvent;
            use wasm_bindgen::{prelude::Closure, JsCast};
            use leptos::prelude::*;
            use js_sys::{Function, JsString};

            // Only initialize once
            if use_context::<SseInitialized>().is_some() {
                leptos::logging::log!("SSE already initialized");
                return Ok(());
            }

            leptos::logging::log!("Initializing SSE connection to: {}", url);
            
            let es = EventSource::new(url)?;
            
            // Add event listeners for debugging
            {
                use wasm_bindgen::JsCast;
                
                // Log when connection opens
                let onopen = Closure::wrap(Box::new(move || {
                    leptos::logging::log!("SSE connection opened successfully");
                }) as Box<dyn Fn()>);
                es.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                onopen.forget();
                
                // Log errors
                let onerror = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    leptos::logging::error!("SSE connection error occurred");
                }) as Box<dyn Fn(_)>);
                es.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onerror.forget();
            }
            
            // Store the EventSource
            EVENT_SOURCE.with(|source| {
                *source.borrow_mut() = Some(es);
            });
            
            // Set up the message handler
            EVENT_SOURCE.with(|source| {
                if let Some(es) = source.borrow().as_ref() {
                    let callback = Closure::wrap(Box::new(move |event: MessageEvent| {
                        leptos::logging::log!("SSE message received");
                        let ws_string = event.data().dyn_into::<JsString>().unwrap().as_string().unwrap();
                        leptos::logging::log!("SSE data: {}", &ws_string);
                        if let Ok(update_signal) = serde_json::from_str::<ServerSignalUpdate>(&ws_string) {
                            let name = &update_signal.name;
                            
                            // Try sync signals first
                            let handled = STATE_SIGNALS.with(|signals| {
                                let handler_map = signals.borrow();
                                if let Some(signal) = handler_map.get(name) {
                                    // Apply any delayed patches first
                                    DELAYED_UPDATES.with(|delayed| {
                                        let mut delayed_map = delayed.borrow_mut();
                                        if let Some(delayed_patches) = delayed_map.remove(name) {
                                            signal.update(|doc| {
                                                for patch in delayed_patches {
                                                    json_patch::patch(doc, &patch).unwrap();
                                                }
                                            });
                                        }
                                    });
                                    
                                    // Apply the current patch
                                    signal.update(|doc| {
                                        json_patch::patch(doc, &update_signal.patch).unwrap();
                                    });
                                    true
                                } else {
                                    false
                                }
                            });
                            
                            // If not found in sync signals, try local signals
                            if !handled {
                                let handled_local = STATE_SIGNALS_LOCAL.with(|signals| {
                                    let handler_map = signals.borrow();
                                    if let Some(signal) = handler_map.get(name) {
                                        // Apply any delayed patches first
                                        DELAYED_UPDATES.with(|delayed| {
                                            let mut delayed_map = delayed.borrow_mut();
                                            if let Some(delayed_patches) = delayed_map.remove(name) {
                                                signal.update(|doc| {
                                                    for patch in delayed_patches {
                                                        json_patch::patch(doc, &patch).unwrap();
                                                    }
                                                });
                                            }
                                        });
                                        
                                        // Apply the current patch
                                        signal.update(|doc| {
                                            json_patch::patch(doc, &update_signal.patch).unwrap();
                                        });
                                        true
                                    } else {
                                        false
                                    }
                                });
                                
                                if !handled_local {
                                    leptos::logging::warn!("No local state for update to {}. Queuing patch.", name);
                                    DELAYED_UPDATES.with(|delayed| {
                                        let mut delayed_map = delayed.borrow_mut();
                                        delayed_map.entry(name.clone()).or_default().push(update_signal.patch.clone());
                                    });
                                }
                            }
                        }
                    }) as Box<dyn FnMut(_)>);
                    
                    let function: &Function = callback.as_ref().unchecked_ref();
                    es.set_onmessage(Some(function));

                    // Keep the closure alive for the lifetime of the program
                    callback.forget();
                    
                    leptos::logging::log!("SSE message handler installed");
                }
            });
            
            // Mark SSE as initialized AFTER setting up the handler
            provide_context(SseInitialized);

            Ok(())
        }

        /// Provides access to the underlying EventSource for advanced use cases
        pub fn with_event_source<F, R>(f: F) -> Option<R>
        where
            F: FnOnce(&EventSource) -> R,
        {
            EVENT_SOURCE.with(|source| {
                source.borrow().as_ref().map(|es| f(es))
            })
        }
    } else {
        #[inline]
        fn provide_sse_inner(_url: &str) -> Result<(), JsValue> {
            Ok(())
        }
    }
}