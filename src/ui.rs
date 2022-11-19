use crate::config::Config;
use crate::cursive::traits::View;
use crate::model::{MultiRepoHistory, RepoCommit};
use crate::utils::execute_on_commit;
use crate::views::{DiffView, MainView, SeperatorView};
use cursive::event::{Event, Key};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::traits::Nameable;
use cursive::traits::Resizable;
use cursive::views::{Canvas, LayerPosition, LinearLayout};
use cursive::views::{ResizedView, ViewRef};
use cursive::Cursive;
use cursive::CursiveExt;
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
    let mut diff_view: ViewRef<DiffView> = siv.find_name("diffView").unwrap();
    diff_view.set_commit(&entry);

    let mut main_view: ViewRef<MainView> = siv.find_name("mainView").unwrap();
    main_view.update_commit_bar(index, commits, &entry);
}

pub fn show(model: MultiRepoHistory, config: Config) {
    let mut siv = Cursive::default();
    siv.load_toml(include_str!("../assets/style.toml")).unwrap();

    //Postpone the initialization of the UI until cursive is running so we can
    // query the terminal dimensions with screen_size()
    siv.cb_sink()
        .send(Box::new(move |siv| {
            let commits = model.commits.len();
            let repos = model.repos.len();
            let first_commit = if commits > 0 {
                Some(model.commits.get(0).unwrap().clone())
            } else {
                None
            };

            let screen_size = siv.screen_size();

            let mut main_view = MainView::from(model);

            main_view.set_on_select(
                move |siv: &mut Cursive, _row: usize, index: usize, entry: &RepoCommit| {
                    let mut diff_view: ViewRef<DiffView> = siv.find_name("diffView").unwrap();
                    diff_view.set_commit(&entry);
                    let mut main_view: ViewRef<MainView> = siv.find_name("mainView").unwrap();
                    main_view.update_commit_bar(index, commits, &entry);
                },
            );
            let landscape_format = screen_size.x / (screen_size.y * 3) >= 1;
            let layout = if landscape_format {
                LinearLayout::vertical()
                    .child(
                        LinearLayout::horizontal()
                            .child(main_view.with_name("mainView").full_screen())
                            .child(SeperatorView::vertical())
                            .child(ResizedView::with_fixed_width(
                                screen_size.x / 2 - 1,
                                DiffView::empty().with_name("diffView"),
                            )),
                    )
                    .child(build_status_bar(commits, repos, screen_size))
            } else {
                LinearLayout::vertical()
                    .child(main_view.with_name("mainView").full_screen())
                    .child(ResizedView::with_fixed_height(
                        screen_size.y / 2 - 1,
                        DiffView::empty().with_name("diffView"),
                    ))
                    .child(build_status_bar(commits, repos, screen_size))
            };

            siv.add_layer(layout);

            register_custom_commands(&config, siv);

            register_builtin_command('q', siv, |s| {
                s.pop_layer();
                if s.screen().get(LayerPosition::FromBack(0)).is_none() {
                    s.quit();
                }
            });
            register_builtin_command('k', siv, |s| {
                let mut diff_view: ViewRef<DiffView> = s.find_name("diffView").unwrap();
                diff_view.on_event(Event::Key(Key::Up));
            });
            register_builtin_command('j', siv, |s| {
                let mut diff_view: ViewRef<DiffView> = s.find_name("diffView").unwrap();
                diff_view.on_event(Event::Key(Key::Down));
            });

            if let Some(commit) = first_commit {
                update(siv, 0, commits, &commit)
            }
        }))
        .unwrap();

    siv.run(); //this call blocks until UI gets terminated
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
            let diff_view: ViewRef<DiffView> = s.find_name("diffView").unwrap();
            if let Some(commit) = &diff_view.commit() {
                let result =
                    execute_on_commit(&executable, args.as_ref().unwrap_or(&String::new()), commit);
                if let Some(error) = &result.err() {
                    let mut main_view: ViewRef<MainView> = s.find_name("mainView").unwrap();
                    main_view.show_error("Failed to open gitk", error);
                }
            }
        });
    }
}
