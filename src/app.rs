use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/raw-explorer.css"/>

        // sets the document title
        <Title text="data.raw explorer"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let (variant, set_variant) = signal(0u8);
    let json_data = Resource::new(move || variant.get(), load_data);
    let version = Resource::new(|| 0, dump_version);

    view! {
        <h1>"Factorio data.raw explorer "
            <Suspense fallback=move || view! {"(?.?.?)"}>
                {move || {
                    version.get().map(|res| match res {
                        Ok(v) => format!("({v})").into_view(),
                        Err(e) => e.to_string().into_view(),
                    })
                }}
            </Suspense>
        </h1>
        <p>"Select the dump variant to explore: "
            <select on:change:target=move |ev| {
                set_variant.set(ev.target().value().parse().unwrap());
            }>
                <option value="0">"base"</option>
                <option value="1">"space-age"</option>
                <option value="2">"quality"</option>
                <option value="3">"elevated-rails"</option>
            </select>
        </p>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
          {move || {
            json_data.get().map(|res| match res {
                Ok(data) => view! { <JsonViewer val=data start_open=true/> }.into_any(),
                Err(e) => view! { <p>{e.to_string()}</p> }.into_any(),
            })
          }}
        </Suspense>
    }
}

use serde_json::Value as JsonValue;

#[component]
fn JsonViewer(
    #[prop(optional)] key: String,
    val: JsonValue,
    #[prop(optional)] start_open: bool,
) -> impl IntoView {
    let (open, set_open) = signal(start_open);

    let row = match val {
        JsonValue::Null => view! { <JsonKV key=key kind="null" val="null".into()/> }.into_any(),
        JsonValue::Bool(b) => view! { <JsonKV key=key kind="bool" val=b.to_string() /> }.into_any(),
        JsonValue::Number(n) => {
            view! { <JsonKV key=key kind="number" val=n.to_string() /> }.into_any()
        }
        JsonValue::String(s) => {
            view! { <JsonKV key=key kind="text" val=format!("\"{s}\"") /> }.into_any()
        }
        JsonValue::Array(arr) => {
            let len = arr.len();
            let children = move || {
                open.get().then(|| {
                    arr.iter()
                        .enumerate()
                        .map(|(idx, v)| {
                            view! {
                                <JsonViewer key=idx.to_string() val=v.clone()/>
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader key=key kind="array" val=len.to_string() write=set_open />
                <div class="json-children">
                    {children}
                </div>
            }
            .into_any()
        }
        JsonValue::Object(obj) => {
            let children = move || {
                open.get().then(|| {
                    obj.iter()
                        .map(|(k, v)| {
                            view! {
                                <JsonViewer key=k.clone() val=v.clone()/>
                            }
                        })
                        .collect_view()
                })
            };

            view! {
                <JsonCollapsibleHeader key=key kind="object" write=set_open />
                <div class="json-children">
                    {children}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div class="json-row" class:expanded=move || open.get()>
            {row}
        </div>
    }
}

#[component]
fn JsonKV(key: String, kind: &'static str, #[prop(optional)] val: String) -> impl IntoView {
    let val = view! {
        <span class=kind>{val}</span>
    }
    .into_any();

    if key.is_empty() {
        return val;
    }

    view! {
        <span class="key">{key}": "</span>{val}
    }
    .into_any()
}

#[component]
fn JsonCollapsibleHeader(
    key: String,
    kind: &'static str,
    #[prop(optional)] val: String,
    write: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <a on:click=move |_| write.update(|v| *v = !*v)>
            <span class="arrow"/>
            <JsonKV key=key kind=kind val=val/>
        </a>
    }
}

#[server]
pub async fn dump_version(id: u8) -> Result<String, ServerFnError> {
    let version = tokio::fs::read_to_string("dumps/version.txt").await?;
    Ok(version.trim().to_string())
}

#[server]
pub async fn load_data(id: u8) -> Result<JsonValue, ServerFnError> {
    use tokio::io::AsyncReadExt;

    let filename = match id {
        0 => "base.json",
        1 => "space-age.json",
        2 => "quality.json",
        3 => "elevated-rails.json",
        _ => return Err(ServerFnError::ServerError(format!("Invalid id: {id}"))),
    };

    let mut file = tokio::fs::File::open(format!("dumps/{filename}")).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;

    Ok(serde_json::from_slice(&buf)?)
}
