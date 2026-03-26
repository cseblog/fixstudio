use dioxus::prelude::*;
use dioxus::document::eval;

/// Hero landing screen shown before any data is loaded.
#[component]
pub fn Hero() -> Element {
    use_effect(move || {
        eval(r#"
            (function() {
                function animCounter(id, target, dur) {
                    var el = document.getElementById(id);
                    if (!el) return;
                    var t0 = performance.now();
                    (function tick() {
                        var p = Math.min((performance.now() - t0) / dur, 1);
                        var v = Math.round((1 - Math.pow(1 - p, 3)) * target);
                        el.textContent = (p >= 1 ? target : v).toLocaleString('en-US');
                        if (p < 1) requestAnimationFrame(tick);
                    })();
                }
                setTimeout(function() {
                    animCounter('hero-msg-count',  1000000, 1600);
                    animCounter('hero-parse-ms',   150,     1200);
                    animCounter('hero-throughput', 631,     1000);
                }, 150);
            })();
        "#);
    });

    rsx! {
        div { class: "hero",
            div { class: "hero-title",
                span { class: "hero-icon", "⚡" }
                h1 { "AiFIXParser.com" }
                p { "Lightning fast FIX protocol parser & inspector" }
            }
            div { class: "hero-stats",
                div { class: "hero-stat hero-stat-a",
                    div { class: "hero-stat-value", id: "hero-msg-count", "0" }
                    div { class: "hero-stat-unit", "messages" }
                    div { class: "hero-stat-label", "parsed in one shot" }
                }
                div { class: "hero-stat hero-stat-featured",
                    div { class: "hero-stat-value",
                        span { id: "hero-parse-ms", "0" }
                        span { class: "hero-stat-suffix", "ms" }
                    }
                    div { class: "hero-stat-unit", "parse time" }
                    div { class: "hero-stat-label", "for 1,000,000 messages" }
                }
                div { class: "hero-stat hero-stat-b",
                    div { class: "hero-stat-value",
                        span { id: "hero-throughput", "0" }
                        span { class: "hero-stat-suffix", " MiB/s" }
                    }
                    div { class: "hero-stat-unit", "throughput" }
                    div { class: "hero-stat-label", "streaming parse" }
                }
            }
            div { class: "hero-demo",
                div { class: "hero-demo-label",
                    span { "Simulating 1,000,000 FIX message parse…" }
                    span { class: "hero-demo-time", "150 ms" }
                }
                div { class: "hero-bar-track",
                    div { class: "hero-bar-fill" }
                }
            }
            p { class: "hero-hint",
                "Paste FIX data and click "
                span { class: "hero-hint-kbd", "Process" }
                "  ·  "
                span { class: "hero-hint-kbd", "Load file" }
                "  ·  or pick a sample from the toolbar"
            }
        }
    }
}
