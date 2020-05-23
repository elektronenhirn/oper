use crate::config::Config;
use crate::cursive::traits::View;
use crate::model::{MultiRepoHistory, RepoCommit};
use crate::utils::execute_on_commit;
use crate::views::{DiffView, MainView, SeperatorView};
use cursive::event::{Event, Key};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::traits::Boxable;
use cursive::traits::Identifiable;
use cursive::views::{BoxView, ViewRef};
use cursive::views::{Canvas, LayerPosition, LinearLayout};
use cursive::Cursive;
use cursive::XY;
use std::default::Default;

fn build_status_bar(commits: usize, repos: usize, size: XY<usize>) -> impl cursive::view::View {
    Canvas::new((commits, repos, size))
        .with_draw(|(commits, repos, size), printer| {
            let style = ColorStyle::new(
                Color::Dark(BaseColor::Black),
                Color::Light(BaseColor::Black),
            );

            printer.with_style(style, |p| {
                let text_left = format!("Found {} commits across {} repositories", commits, repos);
                let text_right = format!(" [{}x{}]", size.x, size.y);
                p.print((0, 0), &text_left);
                let gap: i32 = p.size.x as i32 - text_left.len() as i32 - text_right.len() as i32;
                if gap > 0 {
                    p.print_hline((text_left.len(), 0), gap as usize, " ");
                    p.print((text_left.len() + gap as usize, 0), &text_right);
                }
            });
        })
        .with_required_size(|_model, req| cursive::Vec2::new(req.x, 1))
}

fn update(siv: &mut Cursive, index: usize, commits: usize, entry: &RepoCommit) {
    let mut diff_view: ViewRef<DiffView> = siv.find_id("diffView").unwrap();
    diff_view.set_commit(&entry);

    let mut main_view: ViewRef<MainView> = siv.find_id("mainView").unwrap();
    main_view.update_commit_bar(index, commits, &entry);
}

pub fn show(model: MultiRepoHistory, config: &Config) {
    let commits = model.commits.len();
    let repos = model.repos.len();
    let first_commit = if commits > 0 {
        Some(model.commits.get(0).unwrap().clone())
    } else {
        None
    };

    let mut siv = Cursive::default();
    let screen_size = siv.screen_size();

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
                    .child(SeperatorView::vertical())
                    .child(BoxView::with_fixed_width(
                        screen_size.x / 2 - 1,
                        DiffView::empty().with_id("diffView"),
                    )),
            )
            .child(build_status_bar(commits, repos, screen_size))
    } else {
        LinearLayout::vertical()
            .child(main_view.with_id("mainView").full_screen())
            .child(BoxView::with_fixed_height(
                screen_size.y / 2 - 1,
                DiffView::empty().with_id("diffView"),
            ))
            .child(build_status_bar(commits, repos, screen_size))
    };

    siv.add_layer(layout);

    register_custom_commands(config, &mut siv);

    register_builtin_command('q', &mut siv, |s| {
        s.pop_layer();
        if s.screen().get(LayerPosition::FromBack(0)).is_none() {
            s.quit();
        }
    });
    register_builtin_command('k', &mut siv, |s| {
        let mut diff_view: ViewRef<DiffView> = s.find_id("diffView").unwrap();
        diff_view.on_event(Event::Key(Key::Up));
    });
    register_builtin_command('j', &mut siv, |s| {
        let mut diff_view: ViewRef<DiffView> = s.find_id("diffView").unwrap();
        diff_view.on_event(Event::Key(Key::Down));
    });

    if let Some(commit) = first_commit {
        update(&mut siv, 0, commits, &commit)
    }
    siv.run();
}

fn register_builtin_command<F>(ch: char, siv: &mut Cursive, cb: F)
where
    F: FnMut(&mut Cursive) + 'static,
{
    siv.clear_global_callbacks(ch); //to avoid that custom commands are taking over one of our builtin shortcuts
    siv.add_global_callback(ch, cb);
}

fn register_custom_commands(config: &Config, siv: &mut Cursive) {
    for cmd in &config.custom_command {
        let executable = cmd.executable.clone();
        let args = cmd.args.clone();

        siv.add_global_callback(cmd.key, move |s| {
            let diff_view: ViewRef<DiffView> = s.find_id("diffView").unwrap();
            if let Some(commit) = &diff_view.commit() {
                let result =
                    execute_on_commit(&executable, args.as_ref().unwrap_or(&String::new()), commit);
                if let Some(error) = &result.err() {
                    let mut main_view: ViewRef<MainView> = s.find_id("mainView").unwrap();
                    main_view.show_error("Failed to open gitk", error);
                }
            }
        });
    }
}
