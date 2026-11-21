use std::io::{self, Stdout};

use anyhow::{Context, Result, anyhow};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    application::presets::{PRESETS, SearchPreset},
    cli::{parse_range_f32, parse_range_i32},
    domain::criteria::{NumericRange, SearchCriteria},
};

pub struct TuiSelection {
    pub system: String,
    pub criteria: SearchCriteria,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen { Presets, Manual, Confirm }
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field { System, Count, Bio, Planet, Atmosphere, Temperature, Gravity, Pressure }

struct App {
    screen: Screen, manual_mode: bool, selected: usize, focus: usize,
    system: String, count: String, bio: bool,
    planet: String, atmosphere: String, temperature: String, gravity: String, pressure: String,
    error: Option<String>,
}
impl Default for App {
    fn default() -> Self {
        Self { screen: Screen::Presets, manual_mode: false, selected: 0, focus: 0, system: "Sol".into(), count: "5".into(), bio: false,
            planet: "any".into(), atmosphere: "any".into(), temperature: "0-5000".into(), gravity: "0.1-1.0".into(), pressure: "0-10".into(), error: None }
    }
}
impl App {
    fn fields(&self) -> &'static [Field] {
        match self.screen {
            Screen::Presets => &[Field::System, Field::Count, Field::Bio],
            Screen::Manual => &[Field::System, Field::Planet, Field::Atmosphere, Field::Temperature, Field::Gravity, Field::Pressure, Field::Count, Field::Bio],
            Screen::Confirm => &[],
        }
    }
    fn focused(&self) -> Option<Field> { self.fields().get(self.focus).copied() }
    fn move_focus(&mut self, d: isize) { let n=self.fields().len(); if n>0 { self.focus=(self.focus as isize+d).rem_euclid(n as isize) as usize; } self.error=None; }
    fn edit(&mut self, code: KeyCode) {
        let field=match self.focused(){Some(v)=>v,None=>return};
        if field==Field::Bio { if matches!(code,KeyCode::Char(' ')|KeyCode::Enter){self.bio=!self.bio;} return; }
        let value=match field { Field::System=>&mut self.system,Field::Count=>&mut self.count,Field::Planet=>&mut self.planet,Field::Atmosphere=>&mut self.atmosphere,Field::Temperature=>&mut self.temperature,Field::Gravity=>&mut self.gravity,Field::Pressure=>&mut self.pressure,Field::Bio=>return };
        match code { KeyCode::Char(c)=>value.push(c),KeyCode::Backspace=>{value.pop();},KeyCode::Delete=>value.clear(),_=>{} }
        self.error=None;
    }
    fn load_preset(&mut self) {
        let p=&PRESETS[self.selected];
        self.planet=p.surface_types.join("|"); self.atmosphere=p.atmosphere.into();
        self.temperature=format!("{:.0}-{:.0}",p.temperature.min,p.temperature.max);
        self.gravity="0.1-1.0".into(); self.pressure=format!("{}-{}",p.pressure.min,p.pressure.max);
        self.screen=Screen::Presets; self.manual_mode=false; self.focus=0; self.error=None;
    }
    fn selection(&self)->Result<TuiSelection>{
        if self.system.trim().is_empty(){return Err(anyhow!("Укажите центральную систему"));}
        let count=self.count.trim().parse::<usize>().context("Количество результатов должно быть целым числом")?;
        if count==0{return Err(anyhow!("Количество результатов должно быть больше нуля"));}
        let criteria=if self.manual_mode {
            let (a,b)=parse_range_i32(&self.temperature).context("Температура: диапазон min-max в K")?;
            let (c,d)=parse_range_f32(&self.gravity).context("Гравитация: диапазон min-max в g")?;
            let (e,f)=parse_range_f32(&self.pressure).context("Давление: диапазон min-max в atm")?;
            let mut c=SearchCriteria::from_cli(&self.planet,&self.atmosphere,
                NumericRange::new(a.into(),b.into()).ok_or_else(||anyhow!("Некорректная температура"))?,
                NumericRange::new(c.into(),d.into()).ok_or_else(||anyhow!("Некорректная гравитация"))?,
                NumericRange::new(e.into(),f.into()).ok_or_else(||anyhow!("Некорректное давление"))?,count)?;
            if self.planet.contains('|'){c.planet_types=self.planet.split('|').map(str::trim).filter(|s|!s.is_empty()).map(str::to_owned).collect();c.planet_type=c.planet_types.first().cloned().unwrap_or_else(||"any".into());}
            c.with_bio_signal_exclusion(self.bio)
        } else { PRESETS[self.selected].criteria(count,self.bio)? };
        Ok(TuiSelection{system:self.system.trim().into(),criteria})
    }
}

struct TerminalSession { terminal: Terminal<CrosstermBackend<Stdout>> }
impl TerminalSession {
    fn enter()->Result<Self>{
        enable_raw_mode().context("Не удалось включить raw mode")?;
        let mut out=io::stdout();
        if let Err(e)=execute!(out,EnterAlternateScreen){let _=disable_raw_mode();return Err(e.into());}
        match Terminal::new(CrosstermBackend::new(out)){
            Ok(mut terminal)=>{terminal.hide_cursor()?;Ok(Self{terminal})}
            Err(e)=>{let mut out=io::stdout();let _=execute!(out,LeaveAlternateScreen);let _=disable_raw_mode();Err(e.into())}
        }
    }
}
impl Drop for TerminalSession { fn drop(&mut self){let _=self.terminal.show_cursor();let _=execute!(self.terminal.backend_mut(),LeaveAlternateScreen);let _=disable_raw_mode();} }

pub fn run_tui()->Result<Option<TuiSelection>>{
    let mut term=TerminalSession::enter()?;let mut app=App::default();
    loop{
        term.terminal.draw(|f|draw(f,&app))?;
        if let Event::Key(k)=event::read()?{
            if k.kind!=KeyEventKind::Press{continue;}
            if app.screen==Screen::Confirm{
                match k.code{KeyCode::Enter=>return app.selection().map(Some),KeyCode::Esc|KeyCode::Char('n')=>{app.screen=Screen::Presets;app.error=None;},KeyCode::Char('q')=>return Ok(None),_=>{}}
                continue;
            }
            match k.code{
                KeyCode::Char('q') if app.screen==Screen::Presets=>return Ok(None),
                KeyCode::Esc=>{if app.screen==Screen::Manual{app.screen=Screen::Presets;}else{return Ok(None);}app.error=None;},
                KeyCode::Up if app.screen==Screen::Presets&&app.focus==0=>app.selected=(app.selected+PRESETS.len()-1)%PRESETS.len(),
                KeyCode::Down if app.screen==Screen::Presets&&app.focus==0=>app.selected=(app.selected+1)%PRESETS.len(),
                KeyCode::Tab=>app.move_focus(1),KeyCode::BackTab=>app.move_focus(-1),
                KeyCode::F(5)=>match app.selection(){Ok(_)=>{app.screen=Screen::Confirm;app.error=None;},Err(e)=>app.error=Some(e.to_string())},
                KeyCode::Char('m') if app.screen==Screen::Presets=>{app.screen=Screen::Manual;app.manual_mode=true;app.focus=0;app.error=None;},
                KeyCode::Enter if app.screen==Screen::Presets&&app.focus==0=>app.load_preset(),
                code=>app.edit(code),
            }
        }
    }
}

fn draw(f:&mut ratatui::Frame<'_>,app:&App){
    let area=f.area();
    let root=Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3),Constraint::Min(8),Constraint::Length(3)]).split(area);
    f.render_widget(Paragraph::new("ED: PLANET SEARCH  /  EXOBIOLOGY SCANNER").alignment(Alignment::Center).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)).block(Block::default().borders(Borders::ALL)),root[0]);
    let wide=area.width>=90;
    let cols=if wide{Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(38),Constraint::Percentage(62)]).split(root[1])}else{Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(45),Constraint::Percentage(55)]).split(root[1])};
    let items=PRESETS.iter().enumerate().map(|(i,p)|ListItem::new(format!("{}{:02}  {}",if i==app.selected{"▶ "}else{"  "},p.number,p.name)).style(if i==app.selected{Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)}else{Style::default().fg(Color::Gray)})).collect::<Vec<_>>();
    let mut state=ListState::default();state.select(Some(app.selected));
    f.render_stateful_widget(List::new(items).block(Block::default().title(" ПРОФИЛИ ↑/↓ ").borders(Borders::ALL)),cols[0],&mut state);
    let right=if wide{Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(48),Constraint::Percentage(52)]).split(cols[1])}else{cols.clone()};
    draw_details(f,right[0],&PRESETS[app.selected]);draw_form(f,right[1],app);
    f.render_widget(Paragraph::new("↑/↓ профиль  Tab поле  Space переключатель  m ручная настройка  F5 поиск  Esc назад  q выход").alignment(Alignment::Center).style(Style::default().fg(Color::Gray)).block(Block::default().borders(Borders::ALL)),root[2]);
    if let Some(e)=&app.error{let r=centered(70,22,area);f.render_widget(Clear,r);f.render_widget(Paragraph::new(e.as_str()).wrap(Wrap{trim:true}).block(Block::default().title(" Ошибка ").borders(Borders::ALL).border_style(Style::default().fg(Color::Red))),r);}
    if app.screen==Screen::Confirm{
        let r=centered(72,55,area);f.render_widget(Clear,r);
        let p=&PRESETS[app.selected];
        let summary=format!("Система: {}\nПрофиль: {}\nПоверхность: {}\nАтмосфера: {}\nТемпература: {} K\nГравитация: {} g\nДавление: {} atm\nКоличество: {}\nИсключать биосигналы: {}\n\nEnter — начать поиск    Esc — назад",app.system,p.name,app.planet,app.atmosphere,app.temperature,app.gravity,app.pressure,app.count,if app.bio{"да"}else{"нет"});
        f.render_widget(Paragraph::new(summary).wrap(Wrap{trim:true}).block(Block::default().title(" ПОДТВЕРЖДЕНИЕ ПОИСКА ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green))),r);
    }
}
fn draw_details(f:&mut ratatui::Frame<'_>,r:Rect,p:&SearchPreset){
    let s=format!("Тип поверхности : {}\nАтмосфера        : {}\nТемпература      : {:.0}–{:.0} K\nДавление         : {:.3}–{:.3} atm\nГравитация       : 0.1–1.0 g\nВулканизм        : не фильтруется\n\nСправочно: до {} дорогих видов\nПотенциал: {} млн CR",p.surface_types.join(" ИЛИ "),p.atmosphere,p.temperature.min,p.temperature.max,p.pressure.min,p.pressure.max,p.expensive_species,p.potential_value_cr/1_000_000);
    f.render_widget(Paragraph::new(s).wrap(Wrap{trim:true}).block(Block::default().title(" ПАРАМЕТРЫ ПРОФИЛЯ ").borders(Borders::ALL)),r);
}
fn draw_form(f:&mut ratatui::Frame<'_>,r:Rect,a:&App){
    let mut fields=vec![("Центральная система",&a.system,Field::System)];
    if a.screen==Screen::Manual{fields.extend([("Тип поверхности",&a.planet,Field::Planet),("Атмосфера",&a.atmosphere,Field::Atmosphere),("Температура, K",&a.temperature,Field::Temperature),("Гравитация, g",&a.gravity,Field::Gravity),("Давление, atm",&a.pressure,Field::Pressure)]);}
    fields.push(("Количество результатов",&a.count,Field::Count));
    let mut lines=fields.into_iter().map(|(label,value,field)|Line::from(Span::styled(format!(" {:<23} {}",label,value),if a.focused()==Some(field){Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)}else{Style::default().fg(Color::White)}))).collect::<Vec<_>>();
    lines.push(Line::from(Span::styled(format!(" [ {} ] Исключать тела с биосигналами",if a.bio{"x"}else{" "}),if a.focused()==Some(Field::Bio){Style::default().fg(Color::Black).bg(Color::Cyan)}else{Style::default().fg(Color::Yellow)})));
    if a.screen==Screen::Manual{lines.push(Line::from(Span::styled("Тип/атмосфера: используйте any для отключения фильтра",Style::default().fg(Color::DarkGray))));}
    f.render_widget(Paragraph::new(lines).block(Block::default().title(if a.screen==Screen::Manual{" РУЧНАЯ НАСТРОЙКА "}else{" ПАРАМЕТРЫ ПОИСКА "}).borders(Borders::ALL)),r);
}
fn centered(x:u16,y:u16,a:Rect)->Rect{
    let v=Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100-y)/2),Constraint::Percentage(y),Constraint::Percentage((100-y)/2)]).split(a);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100-x)/2),Constraint::Percentage(x),Constraint::Percentage((100-x)/2)]).split(v[1])[1]
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn profile_three_keeps_or_surface_types(){let mut a=App::default();a.selected=2;a.load_preset();let s=a.selection().unwrap();assert_eq!(s.criteria.planet_types,vec!["Rocky body","High metal content world"]);assert_eq!(s.criteria.gravity,NumericRange{min:0.1,max:1.0});}
    #[test]fn zero_count_is_rejected(){let mut a=App::default();a.count="0".into();assert!(a.selection().is_err());}
    #[test]fn bio_exclusion_is_independent(){let mut a=App::default();a.bio=true;assert!(a.selection().unwrap().criteria.exclude_with_bio_signals);}
    #[test]fn manual_mode_accepts_any_filters(){let mut a=App::default();a.screen=Screen::Manual;assert!(a.selection().is_ok());}
}
