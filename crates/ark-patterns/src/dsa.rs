//! Data-structure and algorithm-pattern recognition over parsed facts.
//!
//! Rules look at how a variable is *used* (which methods are called on it, in which
//! order) and which names a function body touches. Each finding carries the rule id
//! and the evidence that fired it. Static recognition is a best guess about intent;
//! confidence says how strong the evidence is.

use std::collections::{BTreeMap, BTreeSet};

use ark_link::{ParsedFile, file_id, fn_id};
use ark_parse::{CallFact, FnFact};
use ark_schema::{DsaFinding, DsaKind, DsaOp};

/// One use of a container: which op, where, and how it was written.
#[derive(Clone)]
struct Use {
    op: &'static str,
    line: u32,
    call: String,
    fn_qual: String,
}

const PUSH: &[&str] = &[
    "append",
    "push",
    "push_back",
    "put",
    "put_nowait",
    "offer",
    "add_last",
    "addLast",
    "enqueue",
];
const FRONT_PUSH: &[&str] = &[
    "appendleft",
    "unshift",
    "push_front",
    "addFirst",
    "offerFirst",
];
const FRONT_POP: &[&str] = &[
    "popleft",
    "shift",
    "pop_front",
    "pollFirst",
    "poll",
    "dequeue",
    "removeFirst",
];
const BACK_POP: &[&str] = &["pop_back", "pollLast", "removeLast"];
const PEEK: &[&str] = &[
    "peek",
    "top",
    "front",
    "last",
    "first",
    "peekFirst",
    "peekLast",
];

fn base_of(receiver: &str) -> String {
    let r = receiver
        .trim_start_matches("self.")
        .trim_start_matches("this.")
        .trim_start_matches("&mut ")
        .trim_start_matches('&');
    let r = r.split(['[', '(']).next().unwrap_or(r);
    r.trim().to_owned()
}

fn call_text(c: &CallFact) -> String {
    match &c.receiver {
        Some(r) => format!("{r}.{}", c.callee),
        None => c.callee.clone(),
    }
}

/// Owner of a function for grouping container uses across methods: the class part of
/// `Class.method` / `Type::method`, otherwise the function itself.
fn owner(qual: &str, classes: &BTreeSet<&str>) -> String {
    if let Some((cls, _)) = qual.rsplit_once('.').or_else(|| qual.rsplit_once("::"))
        && classes.contains(cls)
    {
        return format!("class:{cls}");
    }
    format!("fn:{qual}")
}

pub fn analyze(files: &[ParsedFile]) -> Vec<DsaFinding> {
    let mut out = Vec::new();
    for pf in files {
        let path = pf.scan.path.as_str();
        let facts = &pf.facts;
        let mut classes: BTreeSet<&str> = facts.classes.iter().map(|c| c.name.as_str()).collect();
        // Rust impl blocks / Go receivers: `Type::method` qualnames name the type.
        for f in &facts.functions {
            if let Some((t, _)) = f.qualname.rsplit_once("::") {
                classes.insert(t);
            }
        }

        // ---- container uses grouped by (owner, variable)
        let mut groups: BTreeMap<(String, String), Vec<Use>> = BTreeMap::new();
        for f in &facts.functions {
            let own = owner(&f.qualname, &classes);
            for c in &f.calls {
                let Some(r) = c.receiver.as_deref() else {
                    continue;
                };
                let (key_base, op) = classify_use(c, r);
                if let Some(op) = op {
                    groups
                        .entry((own.clone(), key_base))
                        .or_default()
                        .push(Use {
                            op,
                            line: c.line,
                            call: call_text(c),
                            fn_qual: f.qualname.clone(),
                        });
                }
            }
        }
        for ((_own, base), mut uses) in groups {
            uses.sort_by_key(|u| u.line);
            if let Some(finding) = container_finding(path, &base, &uses, facts) {
                out.push(finding);
            }
        }

        // ---- per-function shape rules
        for f in &facts.functions {
            out.extend(shape_findings(path, f));
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

/// Normalize one call on a receiver into (variable, op).
fn classify_use(c: &CallFact, receiver: &str) -> (String, Option<&'static str>) {
    let callee = c.callee.as_str();
    // heapq.heappush(h, x) / heap.Push(h, x): the container is the first argument.
    if matches!(receiver, "heapq" | "heap") {
        let var = c
            .ident_args
            .first()
            .map(|a| base_of(a))
            .unwrap_or_else(|| "heap".to_owned());
        let op = match callee {
            "heappush" | "Push" => Some("insert"),
            "heappop" | "Pop" => Some("extract"),
            "heapify" | "Init" => Some("heapify"),
            "heapreplace" | "heappushpop" | "Fix" => Some("replace"),
            "nlargest" | "nsmallest" => Some("peek"),
            _ => None,
        };
        return (format!("heap:{var}"), op);
    }
    let base = base_of(receiver);
    let op = if PUSH.contains(&callee) {
        Some("push")
    } else if FRONT_PUSH.contains(&callee) {
        Some("push_front")
    } else if FRONT_POP.contains(&callee) {
        Some("pop_front")
    } else if BACK_POP.contains(&callee) {
        Some("pop")
    } else if callee == "pop" {
        // `pop()` takes from the end; `pop(0)` from the front; `pop(key)` is a dict.
        match c.n_args {
            0 => Some("pop"),
            1 => Some("pop_arg"),
            _ => None,
        }
    } else if callee == "get" && c.n_args == 0 {
        Some("pop_front") // queue.Queue.get()
    } else if PEEK.contains(&callee) && c.n_args == 0 {
        Some("peek")
    } else if matches!(callee, "find" | "union" | "unite") {
        Some(if callee == "find" { "find" } else { "union" })
    } else {
        None
    };
    (base, op)
}

fn container_finding(
    path: &str,
    base: &str,
    uses: &[Use],
    facts: &ark_parse::FileFacts,
) -> Option<DsaFinding> {
    let has = |op: &str| uses.iter().any(|u| u.op == op);
    let lname = base.to_ascii_lowercase();
    let idents: BTreeSet<&str> = facts
        .functions
        .iter()
        .filter(|f| uses.iter().any(|u| u.fn_qual == f.qualname))
        .flat_map(|f| f.idents.iter().map(String::as_str))
        .collect();
    let typed = |names: &[&str]| names.iter().any(|n| idents.contains(n));

    let (kind, rule, conf, subject) = if let Some(var) = base.strip_prefix("heap:") {
        (DsaKind::Heap, "dsa.heap.heapq", 0.9, var.to_owned())
    } else if has("push")
        && has("pop")
        && (typed(&["BinaryHeap", "PriorityQueue", "MinHeap", "MaxHeap"])
            || lname.contains("heap")
            || lname == "pq")
    {
        (DsaKind::Heap, "dsa.heap.typed", 0.85, base.to_owned())
    } else if (has("push") || has("push_front"))
        && has("pop_front")
        && (has("push_front") || has("pop"))
    {
        (DsaKind::Deque, "dsa.deque.both-ends", 0.85, base.to_owned())
    } else if (has("push") && has("pop_front"))
        || (has("push") && has("pop_arg") && (lname.contains('q') || typed(&["deque"])))
    {
        (DsaKind::Queue, "dsa.queue.fifo", 0.85, base.to_owned())
    } else if has("push") && has("pop") {
        (DsaKind::Stack, "dsa.stack.lifo", 0.85, base.to_owned())
    } else if (has("push") || has("pop")) && lname.contains("stack") {
        (DsaKind::Stack, "dsa.stack.named", 0.6, base.to_owned())
    } else if (has("push") || has("pop_front")) && (lname.contains("queue") || lname == "q") {
        (DsaKind::Queue, "dsa.queue.named", 0.6, base.to_owned())
    } else if has("find") && has("union") {
        (
            DsaKind::UnionFind,
            "dsa.unionfind.calls",
            0.8,
            base.to_owned(),
        )
    } else {
        return None;
    };
    let first = &uses[0];
    let fid = fn_id(path, &first.fn_qual);
    let ops: Vec<DsaOp> = uses
        .iter()
        .filter(|u| u.op != "pop_arg" || kind == DsaKind::Queue)
        .take(64)
        .map(|u| DsaOp {
            op: match (kind, u.op) {
                (DsaKind::Queue | DsaKind::Deque, "push") => "enqueue",
                (DsaKind::Queue | DsaKind::Deque, "pop_front" | "pop_arg") => "dequeue",
                (DsaKind::Heap, "push") => "insert",
                (DsaKind::Heap, "pop") => "extract",
                (_, o) => o,
            }
            .to_owned(),
            line: u.line,
            call: u.call.clone(),
        })
        .collect();
    let evidence = uses
        .iter()
        .take(6)
        .map(|u| format!("{} (line {})", u.call, u.line))
        .collect();
    Some(DsaFinding {
        id: format!("dsa:{fid}:{}:{subject}", kind_slug(kind)),
        kind,
        file_id: file_id(path),
        function_id: fid,
        line: first.line,
        subject: Some(subject),
        rule_id: rule.to_owned(),
        confidence: conf,
        evidence,
        ops,
    })
}

pub fn kind_slug(k: DsaKind) -> &'static str {
    match k {
        DsaKind::Stack => "stack",
        DsaKind::Queue => "queue",
        DsaKind::Deque => "deque",
        DsaKind::Heap => "heap",
        DsaKind::HashMap => "hashMap",
        DsaKind::HashSet => "hashSet",
        DsaKind::LinkedList => "linkedList",
        DsaKind::BinaryTree => "binaryTree",
        DsaKind::Trie => "trie",
        DsaKind::Graph => "graph",
        DsaKind::UnionFind => "unionFind",
        DsaKind::Recursion => "recursion",
        DsaKind::Memoization => "memoization",
        DsaKind::BinarySearch => "binarySearch",
        DsaKind::Sorting => "sorting",
    }
}

fn shape_findings(path: &str, f: &FnFact) -> Vec<DsaFinding> {
    let ids: BTreeSet<&str> = f.idents.iter().map(String::as_str).collect();
    let any = |names: &[&'static str]| -> Option<&'static str> {
        names.iter().find(|n| ids.contains(*n)).copied()
    };
    let all = |names: &[&str]| names.iter().all(|n| ids.contains(n));
    let fid = fn_id(path, &f.qualname);
    let mut out = Vec::new();
    let mut push = |kind: DsaKind,
                    rule: &str,
                    conf: f64,
                    subject: Option<&str>,
                    evidence: Vec<String>,
                    ops: Vec<DsaOp>| {
        out.push(DsaFinding {
            id: format!("dsa:{fid}:{}:{}", kind_slug(kind), subject.unwrap_or("")),
            kind,
            file_id: file_id(path),
            function_id: fid.clone(),
            line: f.line_start,
            subject: subject.map(str::to_owned),
            rule_id: rule.to_owned(),
            confidence: conf,
            evidence,
            ops,
        });
    };

    // Recursion: the function calls itself.
    let self_calls: Vec<&CallFact> = f
        .calls
        .iter()
        .filter(|c| {
            c.callee == f.name
                && c.receiver
                    .as_deref()
                    .is_none_or(|r| matches!(r, "self" | "this" | "Self" | "cls"))
        })
        .collect();
    let recursive = !self_calls.is_empty();
    if recursive {
        let ops = self_calls
            .iter()
            .map(|c| DsaOp {
                op: "recurse".to_owned(),
                line: c.line,
                call: call_text(c),
            })
            .collect();
        let kind_text = if self_calls.len() > 1 {
            "branches into several recursive calls (a recursion tree)"
        } else {
            "calls itself once per step (linear recursion)"
        };
        push(
            DsaKind::Recursion,
            "dsa.recursion.self-call",
            0.95,
            Some(&f.name),
            vec![format!("{} {kind_text}", f.name)],
            ops,
        );
    }

    // Memoization / dynamic programming.
    let memo_dec = f.decorators.iter().find(|d| {
        ["lru_cache", "cache", "memoize", "cached", "functools.cache"]
            .iter()
            .any(|m| d.contains(m))
    });
    if let Some(d) = memo_dec {
        push(
            DsaKind::Memoization,
            "dsa.memo.decorator",
            0.9,
            Some(&f.name),
            vec![d.clone()],
            Vec::new(),
        );
    } else if let Some(m) = any(&["memo", "memoization", "dp", "cache_table", "table"])
        .filter(|m| recursive || *m == "dp")
    {
        {
            push(
                DsaKind::Memoization,
                if recursive {
                    "dsa.memo.table-recursive"
                } else {
                    "dsa.dp.tabulation"
                },
                0.7,
                Some(m),
                vec![format!(
                    "uses `{m}`{}",
                    if recursive { " with recursion" } else { "" }
                )],
                Vec::new(),
            );
        }
    }

    // Binary search vs binary tree: both use left/right.
    let bisect = f.calls.iter().find(|c| {
        matches!(
            c.callee.as_str(),
            "bisect"
                | "bisect_left"
                | "bisect_right"
                | "insort"
                | "binary_search"
                | "binary_search_by"
                | "binarySearch"
                | "partition_point"
        ) || (c.callee == "Search" && c.receiver.as_deref() == Some("sort"))
    });
    let bs_vars = ids.contains("mid")
        && (all(&["lo", "hi"])
            || all(&["low", "high"])
            || all(&["left", "right"])
            || all(&["l", "r"]));
    let tree_vars = all(&["left", "right"])
        && any(&["root", "node", "TreeNode", "val", "Node", "tree"]).is_some();
    if let Some(c) = bisect {
        push(
            DsaKind::BinarySearch,
            "dsa.bsearch.library",
            0.9,
            None,
            vec![format!("{} (line {})", call_text(c), c.line)],
            Vec::new(),
        );
    } else if bs_vars && !tree_vars {
        push(
            DsaKind::BinarySearch,
            "dsa.bsearch.mid",
            0.75,
            None,
            vec!["halves a range with lo/hi/mid".to_owned()],
            Vec::new(),
        );
    }
    if tree_vars {
        let subj = any(&["root", "node", "tree"]);
        push(
            DsaKind::BinaryTree,
            "dsa.tree.left-right",
            if recursive { 0.85 } else { 0.7 },
            subj,
            vec!["follows .left / .right links".to_owned()],
            Vec::new(),
        );
    }

    if ids.contains("children")
        && any(&[
            "is_end",
            "isEnd",
            "is_word",
            "isWord",
            "end_of_word",
            "is_terminal",
            "Trie",
            "TrieNode",
            "trie",
        ])
        .is_some()
    {
        push(
            DsaKind::Trie,
            "dsa.trie.children-end",
            0.8,
            any(&["root", "node"]),
            vec!["walks .children maps character by character".to_owned()],
            Vec::new(),
        );
    }

    if ids.contains("next")
        && any(&["head", "prev", "tail", "ListNode", "LinkedList"]).is_some()
        && !ids.contains("iter")
    {
        push(
            DsaKind::LinkedList,
            "dsa.list.next-links",
            0.65,
            any(&["head", "node"]),
            vec!["follows .next pointers".to_owned()],
            Vec::new(),
        );
    }

    if let Some(g) = any(&[
        "adj",
        "adjacency",
        "adj_list",
        "adjList",
        "graph",
        "neighbors",
        "neighbours",
        "edges",
        "successors",
        "DiGraph",
        "Graph",
    ]) && let Some(v) = any(&[
        "visited",
        "seen",
        "indegree",
        "in_degree",
        "dist",
        "distance",
        "distances",
        "parent",
        "explored",
    ]) {
        let algo = if any(&["popleft", "deque", "queue", "Queue"]).is_some() {
            "breadth-first search"
        } else if any(&["heappush", "heapq", "BinaryHeap", "dist", "distance"]).is_some() {
            "shortest paths (Dijkstra-style)"
        } else if any(&["indegree", "in_degree"]).is_some() {
            "topological sort"
        } else if recursive || ids.contains("stack") {
            "depth-first search"
        } else {
            "graph traversal"
        };
        push(
            DsaKind::Graph,
            "dsa.graph.traversal",
            0.8,
            Some(g),
            vec![format!("`{g}` + `{v}`: {algo}")],
            Vec::new(),
        );
    }

    if ids.contains("parent")
        && ids.contains("find")
        && any(&["union", "rank", "size", "unite"]).is_some()
    {
        push(
            DsaKind::UnionFind,
            "dsa.unionfind.parent-rank",
            0.8,
            Some("parent"),
            vec!["parent[] with find/union".to_owned()],
            Vec::new(),
        );
    }

    if let Some(t) = any(&[
        "defaultdict",
        "Counter",
        "HashMap",
        "BTreeMap",
        "OrderedDict",
        "LinkedHashMap",
        "TreeMap",
    ]) {
        push(
            DsaKind::HashMap,
            "dsa.map.typed",
            0.7,
            Some(t),
            vec![format!("uses {t}")],
            Vec::new(),
        );
    }
    if let Some(t) = any(&["HashSet", "BTreeSet", "TreeSet", "frozenset"]) {
        push(
            DsaKind::HashSet,
            "dsa.set.typed",
            0.7,
            Some(t),
            vec![format!("uses {t}")],
            Vec::new(),
        );
    }

    let lname = f.name.to_ascii_lowercase();
    if (lname.contains("sort") && !lname.contains("sorted_") && f.complexity.max_nesting >= 1)
        || all(&["pivot", "partition"])
    {
        push(
            DsaKind::Sorting,
            "dsa.sort.algorithm",
            0.7,
            Some(&f.name),
            vec![format!("{} implements a sort", f.name)],
            Vec::new(),
        );
    }
    out
}
