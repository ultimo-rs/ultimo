//! Fetch, cache, and search the Ultimo documentation corpus (`llms-full.txt`).
//!
//! The fetch step is a closure so tests can supply a fixed corpus string and
//! never touch the network (mirrors the framework's `JwksClient` pattern).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

type FetchFut = Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send>>;
type FetchFn = Arc<dyn Fn() -> FetchFut + Send + Sync>;
type CorpusCache = Arc<RwLock<Option<(Arc<Corpus>, Instant)>>>;

/// One documentation page, parsed from a `## Heading` section of `llms-full.txt`.
#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub body: String,
}

/// The parsed documentation corpus.
#[derive(Debug, Clone, Default)]
pub struct Corpus {
    pub sections: Vec<Section>,
}

impl Corpus {
    /// Parse `llms-full.txt`. Pages are `## Heading` (H2) blocks; single-`#` lines
    /// are code-block comments, not page boundaries, so we split only on `## `.
    pub fn parse(raw: &str) -> Self {
        let mut sections = Vec::new();
        let mut title: Option<String> = None;
        let mut body = String::new();
        for line in raw.lines() {
            if let Some(h) = line.strip_prefix("## ") {
                if let Some(t) = title.take() {
                    sections.push(Section {
                        title: t,
                        body: body.trim().to_string(),
                    });
                    body.clear();
                }
                title = Some(h.trim().to_string());
            } else if title.is_some() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some(t) = title.take() {
            sections.push(Section {
                title: t,
                body: body.trim().to_string(),
            });
        }
        Corpus { sections }
    }

    /// Rank sections by overlap with the query terms and return the top `n`,
    /// each as its heading plus a snippet of the body.
    pub fn search(&self, query: &str, n: usize) -> String {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .filter(|t| t.len() > 1)
            .collect();
        if terms.is_empty() {
            return "Empty query.".to_string();
        }

        let mut scored: Vec<(usize, &Section)> = self
            .sections
            .iter()
            .map(|s| {
                let title_l = s.title.to_lowercase();
                let body_l = s.body.to_lowercase();
                let score: usize = terms
                    .iter()
                    .map(|t| {
                        // Title matches weigh more than body matches.
                        body_l.matches(t.as_str()).count() + title_l.matches(t.as_str()).count() * 5
                    })
                    .sum();
                (score, s)
            })
            .filter(|(score, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        if scored.is_empty() {
            return format!("No Ultimo docs match '{query}'.");
        }

        let mut out = String::new();
        for (_, s) in scored.into_iter().take(n) {
            out.push_str("## ");
            out.push_str(&s.title);
            out.push('\n');
            out.push_str(&snippet(&s.body, 1200));
            out.push_str("\n\n");
        }
        out.trim_end().to_string()
    }

    /// Return the full body of the page whose title best matches `name`
    /// (case-insensitive: exact, then contains).
    pub fn page(&self, name: &str) -> Option<String> {
        let want = name.trim().to_lowercase();
        self.sections
            .iter()
            .find(|s| s.title.to_lowercase() == want)
            .or_else(|| {
                self.sections
                    .iter()
                    .find(|s| s.title.to_lowercase().contains(&want))
            })
            .map(|s| format!("## {}\n{}", s.title, s.body))
    }
}

fn snippet(body: &str, max: usize) -> String {
    if body.len() <= max {
        body.to_string()
    } else {
        let mut end = max;
        while end < body.len() && !body.is_char_boundary(end) {
            end += 1;
        }
        format!("{}…", &body[..end])
    }
}

/// Fetches the docs corpus on demand and caches the parsed result with a TTL.
#[derive(Clone)]
pub struct DocsCorpus {
    fetch: FetchFn,
    cache: CorpusCache,
    ttl: Duration,
}

impl DocsCorpus {
    /// Construct from a raw fetch closure (used by tests with a fixed corpus).
    pub fn from_fetch<F>(f: F) -> Self
    where
        F: Fn() -> FetchFut + Send + Sync + 'static,
    {
        Self {
            fetch: Arc::new(f),
            cache: Arc::new(RwLock::new(None)),
            ttl: Duration::from_secs(3600),
        }
    }

    /// Fetch the corpus over HTTP from `url` (the deployed `llms-full.txt`).
    pub fn from_url(url: impl Into<String>) -> Self {
        let url = url.into();
        Self::from_fetch(move || {
            let url = url.clone();
            Box::pin(async move {
                let text = reqwest::get(&url).await?.error_for_status()?.text().await?;
                Ok(text)
            })
        })
    }

    /// Return the parsed corpus, fetching (and caching) if needed.
    pub async fn get(&self) -> anyhow::Result<Arc<Corpus>> {
        {
            let guard = self.cache.read().await;
            if let Some((corpus, at)) = guard.as_ref() {
                if at.elapsed() < self.ttl {
                    return Ok(corpus.clone());
                }
            }
        }
        let raw = (self.fetch)().await?;
        let corpus = Arc::new(Corpus::parse(&raw));
        *self.cache.write().await = Some((corpus.clone(), Instant::now()));
        Ok(corpus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Ultimo\n\n> intro\n\n## Server-Sent Events\nUse ctx.sse to push events.\n# not a heading (code comment)\nMore SSE text.\n\n## WebSocket\nRFC 6455 support via app.websocket.\n";

    #[test]
    fn parse_splits_on_h2_only() {
        let c = Corpus::parse(SAMPLE);
        let titles: Vec<_> = c.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, vec!["Server-Sent Events", "WebSocket"]);
        // The `# not a heading` line stays inside the SSE section body.
        assert!(c.sections[0].body.contains("code comment"));
    }

    #[test]
    fn search_ranks_relevant_section_first() {
        let c = Corpus::parse(SAMPLE);
        let out = c.search("sse events", 5);
        assert!(out.starts_with("## Server-Sent Events"), "got: {out}");
    }

    #[test]
    fn search_reports_no_match() {
        let c = Corpus::parse(SAMPLE);
        assert!(c.search("graphql", 5).contains("No Ultimo docs match"));
    }

    #[test]
    fn page_matches_by_title() {
        let c = Corpus::parse(SAMPLE);
        let page = c.page("websocket").unwrap();
        assert!(page.contains("RFC 6455"));
    }

    #[tokio::test]
    async fn docs_corpus_caches_and_serves_injected_corpus() {
        let corpus = DocsCorpus::from_fetch(|| Box::pin(async { Ok(SAMPLE.to_string()) }));
        let c = corpus.get().await.unwrap();
        assert_eq!(c.sections.len(), 2);
    }
}
