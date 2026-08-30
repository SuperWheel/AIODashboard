//! Unified Search：一次查询跨 Tasks / Projects / Notes / Inbox。

use rusqlite::Connection;
use serde::Serialize;

use crate::CoreResult;

const PER_TYPE_LIMIT: i64 = 10;

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub kind: &'static str,
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub total: usize,
    pub hits: Vec<SearchHit>,
}

fn empty_results(query: &str) -> SearchResults {
    SearchResults {
        query: query.to_string(),
        total: 0,
        hits: Vec::new(),
    }
}

pub fn search(conn: &Connection, query: &str) -> CoreResult<SearchResults> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(empty_results(query));
    }

    use dashboard_storage as ds;
    let mut hits = Vec::new();

    for t in ds::task_repo::search(conn, &q, PER_TYPE_LIMIT)? {
        hits.push(SearchHit {
            kind: "task",
            id: t.id,
            title: t.title,
            subtitle: None,
        });
    }
    for p in ds::project_repo::list(conn, true)? {
        if p.name.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q) {
            hits.push(SearchHit {
                kind: "project",
                id: p.id,
                title: p.name,
                subtitle: None,
            });
            if hits.iter().filter(|h| h.kind == "project").count() >= PER_TYPE_LIMIT as usize {
                break;
            }
        }
    }
    for n in ds::note_repo::search(conn, &q, PER_TYPE_LIMIT)? {
        let first_line = n.body.lines().next().unwrap_or("").to_string();
        hits.push(SearchHit {
            kind: "note",
            id: n.id,
            title: if n.title.is_empty() {
                "无标题笔记".into()
            } else {
                n.title
            },
            subtitle: Some(first_line),
        });
    }
    for i in ds::inbox_repo::search(conn, &q, PER_TYPE_LIMIT)? {
        hits.push(SearchHit {
            kind: "inbox",
            id: i.id,
            title: i.content,
            subtitle: None,
        });
    }

    let total = hits.len();
    Ok(SearchResults {
        query: q,
        total,
        hits,
    })
}
