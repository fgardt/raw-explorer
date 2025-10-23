use leptos::{ev::SubmitEvent, html, prelude::*};
use leptos_icons::Icon;
use leptos_router::{
    components::A,
    hooks::{use_navigate, use_params},
};

use crate::app::VariantParams;

#[component]
pub fn ModSelector() -> impl IntoView {
    const WUBE_MODS: [&str; 4] = ["base", "space-age", "quality", "elevated-rails"];
    let params = use_params::<VariantParams>();
    let variant = move || params.read().as_ref().ok().and_then(|p| p.variant.clone());

    let mods = Resource::new(|| (), async |_| get_available_mods().await);

    let other_mod: NodeRef<html::Input> = NodeRef::new();
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        let val = other_mod.get().expect("<input> should be mounted").value();
        if !val.is_empty() {
            let navigate = use_navigate();
            navigate(&format!("/e/{}", val), Default::default());
        }
    };

    view! {
        <div class="mod-select">
            {
                WUBE_MODS.iter().map(|m| {
                    let m = m.to_string();
                    view! {
                        <A href={format!("/e/{}", m.clone())}>
                            {m.clone()}
                        </A>
                    }
                }).collect_view()
            }

            <form on:submit=on_submit>
                <input type="text" list="mods" node_ref=other_mod placeholder="search for other mods" value=move|| {
                    if let Some(v) = variant() && !WUBE_MODS.contains(&v.as_str()) {
                        v
                    } else {
                        "".to_string()
                    }
                }/>
                <datalist id="mods">
                    <Suspense>
                        {move || Suspend::new(async move {
                            let Ok(mods) = mods.await else {
                                return ().into_any();
                            };

                            mods.iter().filter_map(|(name, version)| {
                                if WUBE_MODS.contains(&name.as_str()) {
                                    return None;
                                }

                                let res = view! {
                                    <option value={name.clone()}>{name.clone()} " (" {version.clone()} ")"</option>
                                };

                                Some(res)
                            }).collect_view().into_any()
                        })}
                    </Suspense>
                </datalist>
                <button type="submit">
                    <Icon icon={icondata::FiSearch}/>
                </button>
            </form>
        </div>
    }
}

#[cfg(feature = "ssr")]
#[derive(serde::Serialize, serde::Deserialize)]
struct AvailableMods {
    #[serde(rename = "processed")]
    raw: std::collections::BTreeSet<String>,
}

#[cfg(feature = "ssr")]
impl AvailableMods {
    fn build_list(self) -> Box<[(String, String)]> {
        const WUBE_MODS: [&str; 4] = ["base", "space-age", "quality", "elevated-rails"];

        let mut split = self
            .raw
            .iter()
            .filter_map(|r| {
                let mut parts = r.split('_').collect::<Vec<_>>();
                if parts.len() < 2 {
                    return None;
                }

                let version = parts.pop()?.to_string();
                let name = parts.join("_");

                Some((name, version))
            })
            .collect::<Vec<_>>();

        split.sort_by(|(a, _), (b, _)| {
            let a_wube = WUBE_MODS.iter().position(|&m| m == a).unwrap_or(usize::MAX);
            let b_wube = WUBE_MODS.iter().position(|&m| m == b).unwrap_or(usize::MAX);

            match a_wube.cmp(&b_wube) {
                std::cmp::Ordering::Equal => a.cmp(b),
                other => other,
            }
        });
        split.into_boxed_slice()
    }
}

#[server]
pub async fn get_available_mods() -> Result<Box<[(String, String)]>, ServerFnError> {
    crate::util::fetch_from_resolver::<AvailableMods>("stats")
        .await
        .map(AvailableMods::build_list)
        .map_err(|e| ServerFnError::ServerError(e))
}
