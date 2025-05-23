use leptos::prelude::*;
use leptos_sse::create_sse_signal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

#[component]
pub fn App() -> impl IntoView {
    // Provide SSE connection immediately when the app component is created
    // This needs to happen before any signals are created
    let _ = leptos_sse::provide_sse("/sse");
    
    // Create sse signal after SSE is provided
    let count = create_sse_signal::<Count>("counter");

    view! {
        <div>
            <h1>"Count: " {move || count.get().value.to_string()}</h1>
            <p>"The count should update every second."</p>
            <p style="color: #666; font-size: 0.9em;">
                "If not updating, check the Network tab for an active EventStream connection to /sse"
            </p>
        </div>
    }
}