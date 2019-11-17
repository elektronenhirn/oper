use crate::cursive::traits::View;
use crate::model::{MultiRepoHistory, RepoCommit};
use crate::views::{DiffView, MainView, SeperatorView};
use cursive::event::{Event, Key};
use cursive::theme::ColorStyle;
use cursive::traits::Boxable;
use cursive::traits::Identifiable;
use cursive::views::{Canvas, LayerPosition, LinearLayout};
use cursive::views::{HideableView, IdView, ViewRef};
use cursive::Cursive;
use std::default::Default;
use std::rc::Rc;

fn build_status_bar(model: Rc<String>) -> impl cursive::view::View {
    Canvas::new(model)
        .with_draw(|model, printer| {
            printer.with_style(ColorStyle::tertiary(), |p| p.print((0, 0), &model))
        })
        .with_required_size(|_model, req| cursive::Vec2::new(req.x, 1))
}

fn update(siv: &mut Cursive, index: usize, commits: usize, entry: &RepoCommit) {
    let mut diff_view: ViewRef<DiffView> = siv.find_id("diffView").unwrap();
    diff_view.set_commit(&entry);
    let mut main_view: ViewRef<MainView> = siv.find_id("mainView").unwrap();
    main_view.update_commit_bar(index, commits, &entry);
}

pub fn show(model: MultiRepoHistory) {
    let commits = model.commits.len();
    let repos = model.repos.len();
    let first_commit = if commits > 0 {
        Some(model.commits.get(0).unwrap().clone())
    } else {
        None
    };

    let mut siv = Cursive::default();
    let screen_size = siv.screen_size();

    let status_bar = Rc::new(format!(
        "Found {} commits across {} repositories - [{}x{}]",
        commits, repos, screen_size.x, screen_size.y
    ));

    let mut main_view = MainView::from(model);

    siv.load_toml(include_str!("../assets/style.toml")).unwrap();

    main_view.set_on_select(
        move |siv: &mut Cursive, _row: usize, index: usize, entry: &RepoCommit| {
            let mut diff_view: ViewRef<DiffView> = siv.find_id("diffView").unwrap();
            diff_view.set_commit(&entry);
            let mut main_view: ViewRef<MainView> = siv.find_id("mainView").unwrap();
            main_view.update_commit_bar(index, commits, &entry);
        },
    );
    let landscape_format = screen_size.x / (screen_size.y * 3) >= 1;
    let layout = if landscape_format {
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(main_view.with_id("mainView").full_screen())
                    .child(HideableView::new(SeperatorView::vertical()).hidden().with_id("sepView"))
                    .child(HideableView::new(DiffView::empty().with_id("diffView")).hidden().with_id("diffViewHideable")),
            )
            .child(build_status_bar(status_bar))
    } else {
        LinearLayout::vertical()
            .child(main_view.with_id("mainView").full_screen())
            .child(HideableView::new(DiffView::empty().with_id("diffView")).hidden().with_id("diffViewHideable"))
            .child(build_status_bar(status_bar))
    };

    siv.add_layer(layout);
    siv.add_global_callback(Key::Enter, |s| {
        let mut diff_view: ViewRef<HideableView<IdView<DiffView>>> =
            s.find_id("diffViewHideable").unwrap();
        let sep_view: Option<ViewRef<HideableView<SeperatorView>>> = s.find_id("sepView");
        if diff_view.is_visible() {
            diff_view.hide();
            sep_view.map(|mut v| {v.hide()});
        } else {
            diff_view.unhide();
            sep_view.map(|mut v| {v.unhide()});
        };
    });
    siv.add_global_callback('q', |s| {
        s.pop_layer();
        if s.screen().get(LayerPosition::FromBack(0)).is_none() {
            s.quit();
        }
    });
    siv.add_global_callback('k', |s| {
        let mut diff_view: ViewRef<DiffView> = s.find_id("diffView").unwrap();
        diff_view.on_event(Event::Key(Key::Up));
    });
    siv.add_global_callback('j', |s| {
        let mut diff_view: ViewRef<DiffView> = s.find_id("diffView").unwrap();
        diff_view.on_event(Event::Key(Key::Down));
    });

    first_commit.map(|commit| update(&mut siv, 0, commits, &commit));
    siv.run();
}
