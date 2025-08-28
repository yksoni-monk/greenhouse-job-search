use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio;
use scraper::{Html, Selector};
use std::error::Error;
use std::time::Duration;
use std::io::{self, Write};

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
    selected_job: Option<JobResult>,
}

impl JobApplicationSystem {
    fn new(jobs: Vec<JobResult>) -> Self {
        Self {
            jobs,
            selected_job: None,
        }
    }

    fn display_jobs_for_selection(&self) {
        if self.jobs.is_empty() {
            println!("❌ No jobs available for application.");
            return;
        }

        println!("\n🎯 JOBS AVAILABLE FOR APPLICATION");
        println!("=================================");
        
        for (i, job) in self.jobs.iter().enumerate() {
            println!("{}. 📋 {}", i + 1, job.title);
            println!("   🏢 {} | 📅 {}", job.company, job.date_posted);
            println!();
        }
        
        println!("💡 Enter job number to view details and apply (1-{}), or 'q' to quit", self.jobs.len());
    }

    fn get_user_job_selection(&self) -> Result<Option<usize>, String> {
        print!("Your selection: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| format!("Input error: {}", e))?;
        
        let input = input.trim();
        
        if input.to_lowercase() == "q" || input.to_lowercase() == "quit" {
            return Ok(None);
        }
        
        match input.parse::<usize>() {
            Ok(num) if num >= 1 && num <= self.jobs.len() => Ok(Some(num - 1)),
            Ok(_) => Err(format!("Please enter a number between 1 and {}", self.jobs.len())),
            Err(_) => Err("Please enter a valid number or 'q' to quit".to_string()),
        }
    }

    fn display_job_details(&self, job_index: usize) {
        let job = &self.jobs[job_index];
        
        println!("\n📋 JOB DETAILS");
        println!("================");
        println!("📌 Title: {}", job.title);
        println!("🏢 Company: {}", job.company);
        println!("📅 Date Posted: {}", job.date_posted);
        println!("🔗 URL: {}", job.url);
        println!();
    }

    fn confirm_application(&self, job_index: usize) -> bool {
        let job = &self.jobs[job_index];
        
        println!("🤔 Do you want to apply to this position?");
        println!("   📋 {}", job.title);
        println!("   🏢 {}", job.company);
        
        loop {
            print!("\nApply to this job? (y/n): ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return true,
                "n" | "no" => return false,
                _ => println!("Please enter 'y' for yes or 'n' for no"),
            }
        }
    }

    fn select_and_apply_to_job(&mut self) -> Result<bool, String> {
        loop {
            self.display_jobs_for_selection();
            
            match self.get_user_job_selection()? {
                Some(job_index) => {
                    self.display_job_details(job_index);
                    
                    if self.confirm_application(job_index) {
                        self.selected_job = Some(self.jobs[job_index].clone());
                        
                        // Phase 1: Just show selection confirmation
                        println!("\n✅ JOB SELECTED FOR APPLICATION");
                        println!("📋 {}", self.selected_job.as_ref().unwrap().title);
                        println!("🏢 {}", self.selected_job.as_ref().unwrap().company);
                        println!("\n🚧 Phase 2 (Browser Automation) coming soon...");
                        println!("For now, you can manually apply at: {}", self.selected_job.as_ref().unwrap().url);
                        
                        // Ask if user wants to select another job
                        print!("\nWould you like to select another job for application? (y/n): ");
                        io::stdout().flush().unwrap();
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        
                        if !input.trim().to_lowercase().starts_with('y') {
                            return Ok(true);
                        }
                    } else {
                        println!("❌ Application cancelled. Returning to job list...\n");
                    }
                }
                None => {
                    println!("👋 Exiting job application system...");
                    return Ok(false);
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

    // Search jobs for a specific board token
    async fn search_jobs_for_board(&self, board_token: &str, keyword: &str, location: &str) 
        -> Result<Vec<JobResult>, Box<dyn Error>> {
        Self::search_jobs_for_board_static(&self.client, board_token, keyword, location).await
            .map_err(|e| e.into())
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
    
    // Phase 1: Interactive job selection and application interface
    if !jobs.is_empty() {
        println!("\n🎯 INTERACTIVE JOB APPLICATION");
        println!("Found {} matching jobs. Would you like to apply to any of them?", jobs.len());
        
        print!("Enter job application mode? (y/n): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase().starts_with('y') {
            let mut app_system = JobApplicationSystem::new(jobs);
            
            match app_system.select_and_apply_to_job() {
                Ok(_) => println!("\n✅ Job application session completed!"),
                Err(e) => println!("❌ Error in job application: {}", e),
            }
        } else {
            println!("👋 Search completed. No applications submitted.");
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
