//! Typed navigation. Modules never open each other; they push a `Route`.

/// Where the user can go.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Route {
    Discover,
    /// Items of one category key (`apps`, `installed`, or a catalog category).
    Category(String),
    /// Extensions, Sayri skills, and Sayri plugins.
    Ecosystem(Ecosystem),
    Search(String),
    Item(String),
    Updates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Extensions,
    Sayri,
}

impl Route {
    /// Reads a route from a command line value such as `discover`, `apps`,
    /// `extensions`, `sayri`, `installed`, `updates`, `search:firefox`, or `item:<id>`.
    pub fn from_arg(arg: &str) -> Option<Route> {
        let (head, rest) = arg.split_once(':').map_or((arg, ""), |(h, r)| (h, r));
        match (head, rest) {
            ("discover", "") => Some(Route::Discover),
            ("apps" | "installed", "") => Some(Route::Category(head.to_string())),
            ("extensions", "") => Some(Route::Ecosystem(Ecosystem::Extensions)),
            ("sayri", "") => Some(Route::Ecosystem(Ecosystem::Sayri)),
            ("updates", "") => Some(Route::Updates),
            ("search", q) => Some(Route::Search(q.to_string())),
            ("item", id) if !id.is_empty() => Some(Route::Item(id.to_string())),
            ("category", c) if !c.is_empty() => Some(Route::Category(c.to_lowercase())),
            _ => None,
        }
    }
}

/// A back stack of routes. Pure data; the shell turns changes into pages.
#[derive(Debug, Default)]
pub struct Router {
    stack: Vec<Route>,
}

impl Router {
    pub fn new(start: Route) -> Self {
        Self { stack: vec![start] }
    }

    pub fn current(&self) -> Option<&Route> {
        self.stack.last()
    }

    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Opens `route`. Pushing the current route again does nothing.
    pub fn push(&mut self, route: Route) {
        if self.current() != Some(&route) {
            self.stack.push(route);
        }
    }

    /// Swaps the top route for another, as when search text changes.
    pub fn replace(&mut self, route: Route) {
        self.stack.pop();
        self.stack.push(route);
    }

    /// Replaces the whole stack, as when picking a sidebar entry.
    pub fn reset(&mut self, route: Route) {
        self.stack.clear();
        self.stack.push(route);
    }

    /// Goes back one step and returns the route now on top.
    pub fn pop(&mut self) -> Option<&Route> {
        if self.can_go_back() {
            self.stack.pop();
        }
        self.current()
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_line_routes() {
        assert_eq!(Route::from_arg("discover"), Some(Route::Discover));
        assert_eq!(
            Route::from_arg("apps"),
            Some(Route::Category("apps".into()))
        );
        assert_eq!(
            Route::from_arg("sayri"),
            Some(Route::Ecosystem(Ecosystem::Sayri))
        );
        assert_eq!(
            Route::from_arg("search:fire fox"),
            Some(Route::Search("fire fox".into()))
        );
        assert_eq!(
            Route::from_arg("item:org.x.App"),
            Some(Route::Item("org.x.App".into()))
        );
        assert_eq!(Route::from_arg("item:"), None);
        assert_eq!(Route::from_arg("nope"), None);
    }

    #[test]
    fn push_pop() {
        let mut r = Router::new(Route::Discover);
        assert!(!r.can_go_back());
        r.push(Route::Item("a".into()));
        r.push(Route::Item("b".into()));
        assert_eq!(r.depth(), 3);
        assert_eq!(r.pop(), Some(&Route::Item("a".into())));
        assert_eq!(r.pop(), Some(&Route::Discover));
        assert_eq!(r.pop(), Some(&Route::Discover)); // never empties
    }

    #[test]
    fn pushing_current_route_is_ignored() {
        let mut r = Router::new(Route::Discover);
        r.push(Route::Discover);
        assert_eq!(r.depth(), 1);
    }

    #[test]
    fn replace_swaps_the_top() {
        let mut r = Router::new(Route::Discover);
        r.push(Route::Search("a".into()));
        r.replace(Route::Search("ab".into()));
        assert_eq!(r.depth(), 2);
        assert_eq!(r.current(), Some(&Route::Search("ab".into())));
    }

    #[test]
    fn reset_clears_history() {
        let mut r = Router::new(Route::Discover);
        r.push(Route::Updates);
        r.push(Route::Item("a".into()));
        r.reset(Route::Category("apps".into()));
        assert_eq!(r.depth(), 1);
        assert_eq!(r.current(), Some(&Route::Category("apps".into())));
    }
}
