use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio;
use scraper::{Html, Selector};
use std::error::Error;
use std::time::Duration;
use std::io::{self, Write};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal, Frame,
};

#[derive(Debug, Deserialize, Serialize)]
struct Job {
    id: u64,
    title: String,
    updated_at: String,
    location: JobLocation,
    absolute_url: String,
    departments: Option<Vec<Department>>, // Make this optional
}

#[derive(Debug, Deserialize, Serialize)]
struct JobLocation {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Department {
    id: u64,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobsResponse {
    jobs: Vec<Job>,
}

#[derive(Debug, Clone)]
struct JobResult {
    title: String,
    company: String,
    date_posted: String,
    url: String,
}

struct JobApplicationSystem {
    jobs: Vec<JobResult>,
    list_state: ListState,
    current_view: AppView,
    selected_job_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
enum AppView {
    JobList,
    JobDetails,
    ConfirmApplication,
    ApplicationComplete,
}

impl JobApplicationSystem {
    fn new(jobs: Vec<JobResult>) -> Self {
        let mut list_state = ListState::default();
        if !jobs.is_empty() {
            list_state.select(Some(0));
        }
        
        Self {
            jobs,
            list_state,
            current_view: AppView::JobList,
            selected_job_index: None,
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.jobs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.jobs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_current_job(&mut self) {
        self.selected_job_index = self.list_state.selected();
        self.current_view = AppView::JobDetails;
    }

    fn back_to_list(&mut self) {
        self.current_view = AppView::JobList;
    }

    fn confirm_application(&mut self) {
        self.current_view = AppView::ConfirmApplication;
    }

    fn apply_to_job(&mut self) {
        self.current_view = AppView::ApplicationComplete;
    }

    fn render(&mut self, f: &mut Frame) {
        match self.current_view {
            AppView::JobList => self.render_job_list(f),
            AppView::JobDetails => self.render_job_details(f),
            AppView::ConfirmApplication => self.render_confirm_application(f),
            AppView::ApplicationComplete => self.render_application_complete(f),
        }
    }

    fn render_job_list(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("🎯 JOB BROWSER - Interactive Mode")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Job list
        let items: Vec<ListItem> = self.jobs
            .iter()
            .enumerate()
            .map(|(_i, job)| {
                let content = vec![
                    Line::from(vec![
                        Span::styled("📋 ", Style::default().fg(Color::Blue)),
                        Span::raw(&job.title),
                    ]),
                    Line::from(vec![
                        Span::raw("   🏢 "),
                        Span::styled(&job.company, Style::default().fg(Color::Green)),
                    ]),
                ];
                ListItem::new(content)
            })
            .collect();

        let jobs_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Jobs"))
            .highlight_style(Style::default().bg(Color::LightBlue).fg(Color::Black).add_modifier(Modifier::BOLD))
            .highlight_symbol("→ ");

        f.render_stateful_widget(jobs_list, chunks[1], &mut self.list_state);

        // Controls
        let controls = Paragraph::new("🎮 ↑/↓: Navigate | Enter: View Details | q: Quit")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(controls, chunks[2]);
    }

    fn render_job_details(&mut self, f: &mut Frame) {
        if let Some(index) = self.selected_job_index {
            if let Some(job) = self.jobs.get(index) {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ])
                    .split(f.area());

                // Title
                let title = Paragraph::new("📋 JOB DETAILS")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Cyan));
                f.render_widget(title, chunks[0]);

                // Job details
                let details = vec![
                    Line::from(vec![
                        Span::styled("📌 Title: ", Style::default().fg(Color::Yellow)),
                        Span::raw(&job.title),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🏢 Company: ", Style::default().fg(Color::Green)),
                        Span::raw(&job.company),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("📅 Date Posted: ", Style::default().fg(Color::Blue)),
                        Span::raw(&job.date_posted),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🔗 URL: ", Style::default().fg(Color::Magenta)),
                        Span::raw(&job.url),
                    ]),
                ];

                let details_paragraph = Paragraph::new(details)
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(details_paragraph, chunks[1]);

                // Controls
                let controls = Paragraph::new("🎮 a: Apply | b: Back to List | q: Quit")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(controls, chunks[2]);
            }
        }
    }

    fn render_confirm_application(&mut self, f: &mut Frame) {
        if let Some(index) = self.selected_job_index {
            if let Some(job) = self.jobs.get(index) {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ])
                    .split(f.area());

                // Title
                let title = Paragraph::new("🤔 CONFIRM APPLICATION")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Red));
                f.render_widget(title, chunks[0]);

                // Confirmation details
                let details = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("📋 ", Style::default().fg(Color::Blue)),
                        Span::styled(&job.title, Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🏢 ", Style::default().fg(Color::Green)),
                        Span::raw(&job.company),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🔗 ", Style::default().fg(Color::Magenta)),
                        Span::raw(&job.url),
                    ]),
                    Line::from(""),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Do you want to apply to this position?", Style::default().fg(Color::Yellow)),
                    ]),
                ];

                let details_paragraph = Paragraph::new(details)
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(details_paragraph, chunks[1]);

                // Controls
                let controls = Paragraph::new("🎮 y: Yes, Apply | n: No, Go Back")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(controls, chunks[2]);
            }
        }
    }

    fn render_application_complete(&mut self, f: &mut Frame) {
        if let Some(index) = self.selected_job_index {
            if let Some(job) = self.jobs.get(index) {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ])
                    .split(f.area());

                // Title
                let title = Paragraph::new("✅ JOB SELECTED FOR APPLICATION")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                f.render_widget(title, chunks[0]);

                // Success message
                let details = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("📋 ", Style::default().fg(Color::Blue)),
                        Span::styled(&job.title, Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🏢 ", Style::default().fg(Color::Green)),
                        Span::raw(&job.company),
                    ]),
                    Line::from(""),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("🚧 Phase 2 (Browser Automation) coming soon...", Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                    Line::from("For now, you can manually apply at:"),
                    Line::from(vec![
                        Span::styled(&job.url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
                    ]),
                ];

                let details_paragraph = Paragraph::new(details)
                    .block(Block::default().borders(Borders::ALL))
                    .wrap(ratatui::widgets::Wrap { trim: true });
                f.render_widget(details_paragraph, chunks[1]);

                // Controls
                let controls = Paragraph::new("🎮 Press any key to continue...")
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(controls, chunks[2]);
            }
        }
    }

    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Setup terminal
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        // Cleanup
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        result
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn Error>> {
        if self.jobs.is_empty() {
            println!("❌ No jobs available for application.");
            return Ok(());
        }

        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                match self.current_view {
                    AppView::JobList => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Down => self.next(),
                            KeyCode::Up => self.previous(),
                            KeyCode::Enter => self.select_current_job(),
                            _ => {}
                        }
                    }
                    AppView::JobDetails => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('b') => self.back_to_list(),
                            KeyCode::Char('a') => self.confirm_application(),
                            _ => {}
                        }
                    }
                    AppView::ConfirmApplication => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('y') => self.apply_to_job(),
                            KeyCode::Char('n') => self.back_to_list(),
                            _ => {}
                        }
                    }
                    AppView::ApplicationComplete => {
                        // Any key to continue browsing or quit
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            _ => self.back_to_list(),
                        }
                    }
                }
            }
        }
    }
}

struct GreenhouseJobSearcher {
    client: reqwest::Client,
    board_tokens: HashSet<String>,
}

impl GreenhouseJobSearcher {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            board_tokens: HashSet::new(),
        }
    }

    // Method 1: Search Google for greenhouse board tokens (simplified approach)
    async fn find_board_tokens_via_google(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🔍 Searching for Greenhouse board tokens...");
        
        // Google search query to find greenhouse boards
        let search_query = "site:boards.greenhouse.io";
        let google_url = format!("https://www.google.com/search?q={}&num=100", 
                                urlencoding::encode(search_query));

        match self.client.get(&google_url).send().await {
            Ok(response) => {
                let html = response.text().await?;
                let document = Html::parse_document(&html);
                let link_selector = Selector::parse("a[href*='boards.greenhouse.io']")
                    .map_err(|_| "Failed to parse CSS selector")?;

                for element in document.select(&link_selector) {
                    if let Some(href) = element.value().attr("href") {
                        if let Some(token) = self.extract_board_token(href) {
                            self.board_tokens.insert(token);
                        }
                    }
                }
                
                println!("📋 Found {} board tokens from Google search", self.board_tokens.len());
                
                // Print found tokens for debugging
                if !self.board_tokens.is_empty() {
                    println!("🔍 Board tokens from Google: {:?}", 
                            self.board_tokens.iter().take(10).collect::<Vec<_>>());
                }
            }
            Err(e) => {
                println!("⚠️  Google search failed: {}. Using fallback method.", e);
                self.use_known_board_tokens();
            }
        }

        // If Google search didn't find anything, use fallback
        if self.board_tokens.is_empty() {
            println!("⚠️  No tokens found via Google search. Using fallback method.");
            self.use_known_board_tokens();
        }

        println!("📋 Total board tokens to search: {}", self.board_tokens.len());
        
        // Print some of the tokens we'll be using
        if !self.board_tokens.is_empty() {
            println!("🎯 Sample board tokens: {:?}", 
                    self.board_tokens.iter().take(10).collect::<Vec<_>>());
        }
        
        Ok(())
    }

    // Method 2: Use some known popular board tokens as fallback
    fn use_known_board_tokens(&mut self) {
        // More verified board tokens that are likely to work
        let known_tokens = vec![
            "stripe", "uber", "airbnb", "shopify", "atlassian", 
            "mongodb", "snowflake", "databricks", "plaid", "twilio",
            "coinbase", "square", "dropbox", "slack", "zoom",
            "figma", "notion", "airtable", "zapier", "hubspot",
            "asana", "gitlab", "newrelic", "datadog", "sendgrid",
            // Add some more verified ones
            "doordash", "instacart", "reddit", "discord", "spotify",
            "pinterest", "robinhood", "lyft", "github", "palantir",
        ];

        println!("🔄 Adding {} known board tokens as fallback", known_tokens.len());
        
        for token in known_tokens {
            self.board_tokens.insert(token.to_string());
        }
        
        println!("✅ Fallback tokens added: {:?}", 
                self.board_tokens.iter().take(10).collect::<Vec<_>>());
    }

    // Extract board token from greenhouse URL
    fn extract_board_token(&self, url: &str) -> Option<String> {
        if url.contains("boards.greenhouse.io/") {
            let parts: Vec<&str> = url.split("boards.greenhouse.io/").collect();
            if parts.len() > 1 {
                let token_part = parts[1].split('/').next()?;
                if !token_part.is_empty() && token_part != "embed" {
                    return Some(token_part.to_string());
                }
            }
        }
        None
    }

    // Static version for concurrent execution
    async fn search_jobs_for_board_static(client: &reqwest::Client, board_token: &str, keyword: &str, location: &str) 
        -> Result<Vec<JobResult>, String> {
        
        // Use content=true to get department information
        let api_url = format!("https://boards-api.greenhouse.io/v1/boards/{}/jobs?content=true", board_token);
        
        let response = match client.get(&api_url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    // Print debug info for failed requests occasionally
                    if resp.status() == 404 && rand::random::<f32>() < 0.2 { // 20% chance to print 404s
                        println!("\n🔍 Debug: {} returned status {} (board doesn't exist)", board_token, resp.status());
                    } else if rand::random::<f32>() < 0.1 {
                        println!("\n🔍 Debug: {} returned status {}", board_token, resp.status());
                    }
                    return Ok(vec![]);
                }
                resp
            },
            Err(e) => {
                if rand::random::<f32>() < 0.1 { // 10% chance to print network errors
                    println!("\n🔍 Debug: {} network error: {}", board_token, e);
                }
                return Ok(vec![]);
            }
        };

        let jobs_response: JobsResponse = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                if rand::random::<f32>() < 0.1 { // 10% chance to print JSON errors
                    println!("\n🔍 Debug: {} JSON parse error: {}", board_token, e);
                }
                return Ok(vec![]);
            }
        };

        let mut matching_jobs = Vec::new();
        let total_jobs = jobs_response.jobs.len();
        
        // Always print successful API calls with job counts
        if total_jobs > 0 {
            println!("\n✅ {}: {} jobs found", board_token, total_jobs);
        }
        
        for job in &jobs_response.jobs {
            // More flexible keyword matching - split the search term
            let keyword_lower = keyword.to_lowercase();
            let keywords: Vec<&str> = keyword_lower.split_whitespace().collect();
            let job_title_lower = job.title.to_lowercase();
            
            // Check if job title contains all keywords (more flexible than exact phrase)
            let title_matches = keywords.iter().all(|&kw| {
                job_title_lower.contains(kw) || 
                // Also check for common variations
                (kw == "principal" && (job_title_lower.contains("senior") || job_title_lower.contains("staff") || job_title_lower.contains("lead"))) ||
                (kw == "product" && job_title_lower.contains("product")) ||
                (kw == "manager" && (job_title_lower.contains("manager") || job_title_lower.contains("management")))
            });
            
            // More flexible location matching
            let job_location_lower = job.location.name.to_lowercase();
            let location_matches = 
                job_location_lower.contains(&location.to_lowercase()) ||
                job_location_lower.contains("remote") ||
                job_location_lower.contains("bay area") ||
                job_location_lower.contains("san francisco") ||
                job_location_lower.contains("california") ||
                job_location_lower.contains("ca") ||
                job_location_lower.contains("fremont") ||
                job_location_lower.contains("silicon valley") ||
                job_location_lower.contains("sf") ||
                // Also include broader remote/hybrid options
                job_location_lower.contains("anywhere") ||
                job_location_lower.contains("us") ||
                job_location_lower.contains("united states");
            
            // Print some examples for debugging (first few jobs from each company)
            if matching_jobs.len() < 3 && rand::random::<f32>() < 0.3 {
                println!("🔍 Checking: '{}' at '{}' (title_match: {}, location_match: {})", 
                        job.title, job.location.name, title_matches, location_matches);
            }
            
            if title_matches && location_matches {
                // Try to get company name from departments or use board token
                let company_name = if let Some(departments) = &job.departments {
                    if !departments.is_empty() {
                        departments[0].name.clone()
                    } else {
                        // Capitalize board token
                        board_token.chars().next().unwrap().to_uppercase().collect::<String>() + &board_token[1..]
                    }
                } else {
                    // Capitalize board token
                    board_token.chars().next().unwrap().to_uppercase().collect::<String>() + &board_token[1..]
                };

                println!("\n🎉 MATCH FOUND: '{}' at {} ({})", job.title, company_name, job.location.name);

                matching_jobs.push(JobResult {
                    title: job.title.clone(),
                    company: company_name,
                    date_posted: job.updated_at.clone(),
                    url: job.absolute_url.clone(),
                });
            }
        }

        Ok(matching_jobs)
    }


    // Main search function - now returns jobs for application interface
    async fn search_jobs(&mut self, keyword: &str, location: &str) -> Result<Vec<JobResult>, Box<dyn Error>> {
        println!("🚀 Starting job search...");
        println!("🔍 Keyword: {}", keyword);
        println!("📍 Location: {}", location);
        println!();

        // First, find board tokens
        self.find_board_tokens_via_google().await?;

        let total_boards = self.board_tokens.len();
        println!("🔄 Searching jobs across {} companies concurrently...", total_boards);

        // Create concurrent tasks for all board tokens
        let mut tasks = Vec::new();
        let client = self.client.clone();
        let keyword = keyword.to_string();
        let location = location.to_string();

        for board_token in self.board_tokens.iter() {
            let client = client.clone();
            let board_token = board_token.clone();
            let keyword = keyword.clone();
            let location = location.clone();

            let task = tokio::spawn(async move {
                // Add small delay to be respectful to the API
                tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 200)).await;
                
                Self::search_jobs_for_board_static(&client, &board_token, &keyword, &location).await
            });
            
            tasks.push(task);
        }

        // Wait for all tasks to complete and collect results
        let mut all_jobs = Vec::new();
        let mut completed = 0;
        
        for task in tasks {
            completed += 1;
            print!("\rProgress: {}/{} companies completed", completed, total_boards);
            
            match task.await {
                Ok(Ok(jobs)) => {
                    all_jobs.extend(jobs);
                }
                Ok(Err(e)) => {
                    eprintln!("\n⚠️  Error in search task: {}", e);
                }
                Err(e) => {
                    eprintln!("\n⚠️  Task join error: {}", e);
                }
            }
        }

        println!("\n");
        self.display_results(&all_jobs);
        Ok(all_jobs)
    }

    fn display_results(&self, jobs: &Vec<JobResult>) {
        println!("📊 SEARCH RESULTS");
        println!("=================");
        
        if jobs.is_empty() {
            println!("❌ No jobs found matching your criteria.");
            return;
        }

        println!("✅ Found {} matching job(s):\n", jobs.len());

        for (i, job) in jobs.iter().enumerate() {
            println!("{}. 📋 Job Title: {}", i + 1, job.title);
            println!("   🏢 Company: {}", job.company);
            println!("   📅 Date Posted: {}", job.date_posted);
            println!("   🔗 URL: {}", job.url);
            println!();
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🌱 Greenhouse Job Search & Application Tool");
    println!("==========================================\n");

    let mut searcher = GreenhouseJobSearcher::new();
    
    // Search parameters
    let keyword = "principal product manager";
    let location = "94555"; // Fremont, CA area
    
    // Phase 1: Search for jobs
    let jobs = searcher.search_jobs(keyword, location).await?;
    
    // Phase 1: Interactive job browser
    if !jobs.is_empty() {
        println!("\n✅ SEARCH COMPLETE");
        println!("Found {} matching jobs!", jobs.len());
        
        print!("Enter interactive job browser? (y/n): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase().starts_with('y') {
            let mut app_system = JobApplicationSystem::new(jobs);
            
            match app_system.run() {
                Ok(_) => println!("\n✅ Job browser session completed!"),
                Err(e) => println!("❌ Error in job browser: {}", e),
            }
        } else {
            println!("👋 Search completed. Use interactive browser next time to apply!");
        }
    } else {
        println!("❌ No jobs found. Try different search criteria.");
    }
    
    Ok(())
}

// Add these dependencies to Cargo.toml:
/*
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
scraper = "0.18"
urlencoding = "2.1"
*/
