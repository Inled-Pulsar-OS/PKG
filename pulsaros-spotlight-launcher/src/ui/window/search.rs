//! Search flow: query changes, result rendering, and calculator history.
use super::*;

impl SpotlightWindow {
    pub(super) fn on_search_changed(self: &Rc<Self>) {
        self.nav_apply(NavEvent::QueryChanged);
        let empty = self.search_entry.text().is_empty();
        if self.layout.get() != LayoutState::Panel && !empty {
            self.apply_layout(LayoutState::Panel);
        }
        if empty {
            let typed = std::mem::replace(&mut self.nav.borrow_mut().typed, false);
            let ctx = LayoutCtx { query_typed: typed, ..self.panel_ctx() };
            let o = layout::transition(self.layout.get(), &ctx, LayoutEvent::QueryEmptied);
            if o.action == LayoutAction::Collapse {
                self.reset_to_root();
                return;
            }
        } else {
            self.nav_apply(NavEvent::Typed(true));
        }
        if let Some(id) = self.search_state.debounce_id.borrow_mut().take() {
            id.remove();
        }
        let s = self.clone();
        let id = glib::timeout_add_local(Duration::from_millis(DEBOUNCE_MS), move || {
            // The source has fired: drop the stale handle so a later
            // on_search_changed never removes an already-destroyed source
            *s.search_state.debounce_id.borrow_mut() = None;
            s.do_search(false);
            glib::ControlFlow::Break
        });
        *self.search_state.debounce_id.borrow_mut() = Some(id);
    }

    // ---- searching ---------------------------------------------------------------

    pub(super) fn render(&self, keep: bool) {
        let sections = self.current.borrow().clone();
        self.result_view.set_sections(sections, keep);
    }

    /// Append backend rows to a named section (creating it) and re-render.
    fn append_rows(&self, seq: u64, title: &str, rows: Vec<RowData>) {
        if self.search_state.search_seq.get() != seq || rows.is_empty() {
            return;
        }
        {
            let mut cur = self.current.borrow_mut();
            let have: std::collections::HashSet<String> =
                cur.iter().flat_map(|(_, r)| r.iter().map(|x| x.row.id.clone())).collect();
            let rows: Vec<RowData> = rows.into_iter().filter(|r| !have.contains(&r.row.id)).collect();
            if rows.is_empty() {
                return;
            }
            match cur.iter_mut().find(|(t, _)| t == title) {
                Some((_, existing)) => existing.extend(rows),
                None => cur.push((title.to_string(), rows)),
            }
        }
        self.render(true);
    }

    fn core_sections(&self, query: &str, extra: &[Row]) -> Vec<Section> {
        let commands = self.commands.borrow();
        let data = self
            .search_state
            .search_data
            .borrow_mut()
            .get_or_insert_with(|| {
                let apps: Vec<AppEntry> = self.backend.apps_snapshot().iter().map(app_entry).collect();
                let index = SearchIndex::new(&apps, &commands);
                Rc::new((apps, index))
            })
            .clone();
        let settings = self.settings.borrow();
        let frecency = self.frecency.borrow();
        let states = self.search_state.states.borrow();
        let ctx = Ctx {
            apps: &data.0,
            commands: &commands,
            states: &states,
            settings: &settings,
            frecency: &frecency,
            now: now_secs(),
            extra,
        };
        RootSearch::sections_indexed(query, &ctx, &data.1)
    }

    pub(super) fn do_search(self: &Rc<Self>, keep: bool) {
        // A bar state draws no results: skip the root search (also from state probes and app updates).
        if self.layout.get() != LayoutState::Panel {
            return;
        }
        let seq = self.search_state.search_seq.get() + 1;
        self.search_state.search_seq.set(seq);

        let query = self.search_entry.text().to_string();
        let category = self.category.get();
        if self.search_view(&query, keep) {
            return;
        }

        // If in media/document category and query empty and not browsing, start browsing the category dir
        if query.trim().is_empty() && self.nav.borrow().is_root() {
            if let Some(d) = category.browse_dir() {
                if d.is_dir() {
                    self.nav_apply(NavEvent::OpenDir(d.display().to_string()));
                }
            }
        }

        let browsing_dir = self.nav.borrow().dir().map(str::to_string);
        if let Some(dir_path) = browsing_dir.as_ref() {
            let rows = browse_directory_plain(dir_path, query.trim());
            *self.current.borrow_mut() = vec![("Files".to_string(), rows)];
            self.render(keep);
            return;
        }

        let is_empty = query.trim().is_empty();
        if self.in_calc_history() {
            let rows = CalcHistoryProvider::new(self.search_state.calc_history.clone()).instant(&Query::new(&query));
            *self.current.borrow_mut() = vec![("Calculator History".to_string(), self.attach_all(rows, &None))];
            self.render(keep);
            return;
        }
        let sections: SectionList = match category {
            Category::All | Category::Applications | Category::Actions => {
                let show_extra = category == Category::All && !is_empty;
                let calc = if show_extra { Calculator::calculate(&query) } else { None };
                let q = Query::new(&query);
                let mut registry = Registry::default();
                registry.register(Box::new(CalculatorProvider));
                if show_extra {
                    registry.register(Box::new(self.clipboard_provider(3)));
                }
                let mut extra = registry.instant(&q);
                if !show_extra {
                    extra.retain(|r| r.kind == RowKind::Command);
                }
                let run = {
                    let (this, q) = (self.clone(), query.clone());
                    Rc::new(RootRun::new(move || this.core_sections(&q, &extra)))
                };
                let mut secs: SectionList = run
                    .sections()
                    .iter()
                    .map(|s| {
                        (s.title.clone(), self.attach_all(s.rows.clone(), &calc))
                    })
                    .collect();
                if show_extra {
                    self.add_extra_rows(&query, &mut secs);
                }
                if calc.is_some() {
                    secs.retain(|(t, _)| t != "Search the web");
                }
                if category != Category::All {
                    let provider: Box<dyn Provider> = if category == Category::Actions {
                        Box::new(CommandsProvider::new(run.clone()))
                    } else {
                        Box::new(AppsProvider::new(run.clone()))
                    };
                    let mut registry = Registry::default();
                    registry.register(provider);
                    let ids: std::collections::HashSet<String> =
                        registry.instant(&Query::new(&query)).into_iter().map(|r| r.id).collect();
                    for (_, rows) in secs.iter_mut() {
                        rows.retain(|r| ids.contains(&r.row.id));
                    }
                    secs.retain(|(t, rows)| !rows.is_empty() && t != "Favorites" && t != "Suggestions");
                }
                secs
            }
            Category::Clipboard => {
                let rows = self.clipboard_provider(usize::MAX).instant(&Query::new(&query));
                vec![("Clipboard".to_string(), self.attach_all(rows, &None))]
            }
            Category::Calc => {
                let calc_query = if is_empty { "1+1" } else { &query };
                let mut rows = CalculatorProvider.instant(&Query::new(calc_query));
                rows.retain(|r| r.kind == RowKind::Calc);
                rows.extend(CalcHistoryProvider::new(self.search_state.calc_history.clone()).instant(&Query::new(&query)));
                vec![("Calculator".to_string(), self.attach_all(rows, &Calculator::calculate(calc_query)))]
            }
            _ => Vec::new(),
        };
        // Files, web and history decide for themselves whether the category and query apply.
        let q = Query::new(&query);
        let mut registry = Registry::default();
        registry.register(Box::new(WebProvider::new(category)));
        registry.register(Box::new(FilesProvider::new(category)));
        registry.register(Box::new(HistoryProvider::new(category)));
        let mut sections = sections;
        let web = registry.instant(&q);
        if !web.is_empty() {
            sections.push(("Web".to_string(), web.into_iter().map(file_data).collect()));
        }
        *self.current.borrow_mut() = sections;
        self.render(keep);

        for job in registry.background(&q) {
            self.loading_begin();
            let (s, title) = (self.clone(), job.title);
            run_job(job, move |rows| {
                s.loading_end();
                s.append_rows(seq, title, rows);
            });
        }
    }

    fn clipboard_provider(&self, limit: usize) -> ClipboardProvider {
        let mgr = self.backend.clipboard();
        let items = move || mgr.as_ref().map(|m| m.get_items().into_iter().map(|i| i.text).collect()).unwrap_or_default();
        ClipboardProvider::new(items, limit)
    }

    /// Rows plus what activating them needs: the clipboard payload, or the calculator result
    /// (`live` for a calculation, the saved row for a history entry).
    fn attach_all(&self, rows: Vec<Row>, live: &Option<CalcResult>) -> Vec<RowData> {
        rows.into_iter()
            .map(|row| {
                let payload = (row.kind == RowKind::Clipboard)
                    .then(|| Payload { url: row.id.clone(), mime: clipboard_provider::MIME.to_string(), app: None });
                let calc = match (row.kind, row.id.strip_prefix("calchist:")) {
                    (RowKind::Calc, Some(id)) => {
                        id.parse().ok().and_then(|id| self.search_state.calc_history.borrow().get(id)).map(|h| history_result(&h))
                    }
                    (RowKind::Calc, None) => live.clone(),
                    _ => None,
                };
                RowData { row, payload, calc }
            })
            .collect()
    }

    pub(super) fn show_history(self: &Rc<Self>) {
        let rows: Vec<RowData> = self
            .settings
            .borrow()
            .history()
            .iter()
            .map(|q| {
                let r = Row::simple(RowKind::Fallback, &format!("history:{q}"), q, "", "document-open-recent-symbolic");
                RowData { row: r, payload: None, calc: None }
            })
            .collect();
        if rows.is_empty() {
            return;
        }
        self.nav_apply(NavEvent::ShowRecent);
        *self.current.borrow_mut() = vec![("Recent searches".to_string(), rows)];
        self.render(false);
    }

    pub(super) fn enter_calc_history(self: &Rc<Self>) {
        self.close_panel();
        self.nav_apply(NavEvent::ShowCalcHistory);
        self.search_entry.set_placeholder_text(Some("Search calculator history\u{2026}"));
        self.search_entry.set_text("");
        self.do_search(false);
    }
}
