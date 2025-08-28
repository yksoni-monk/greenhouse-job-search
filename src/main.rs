use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio;
use scraper::{Html, Selector};
use std::error::Error;
use std::time::Duration;

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

#[derive(Debug)]
struct JobResult {
    title: String,
    company: String,
    date_posted: String,
    url: String,
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

    // Search jobs for a specific board token
    async fn search_jobs_for_board(&self, board_token: &str, keyword: &str, location: &str) 
        -> Result<Vec<JobResult>, Box<dyn Error>> {
        
        // Use content=true to get department information
        let api_url = format!("https://boards-api.greenhouse.io/v1/boards/{}/jobs?content=true", board_token);
        
        let response = match self.client.get(&api_url).send().await {
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

    // Main search function
    async fn search_jobs(&mut self, keyword: &str, location: &str) -> Result<(), Box<dyn Error>> {
        println!("🚀 Starting job search...");
        println!("🔍 Keyword: {}", keyword);
        println!("📍 Location: {}", location);
        println!();

        // First, find board tokens
        self.find_board_tokens_via_google().await?;

        let mut all_jobs = Vec::new();
        let total_boards = self.board_tokens.len();
        let mut processed = 0;

        println!("🔄 Searching jobs across {} companies...", total_boards);

        for board_token in &self.board_tokens.clone() {
            processed += 1;
            print!("\rProgress: {}/{} - Checking {}...", processed, total_boards, board_token);
            
            match self.search_jobs_for_board(board_token, keyword, location).await {
                Ok(jobs) => {
                    all_jobs.extend(jobs);
                }
                Err(e) => {
                    eprintln!("\n⚠️  Error searching {}: {}", board_token, e);
                }
            }

            // Add small delay to be respectful to the API
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        println!("\n");
        self.display_results(all_jobs);
        Ok(())
    }

    fn display_results(&self, jobs: Vec<JobResult>) {
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
    println!("🌱 Greenhouse Job Search Tool");
    println!("==============================\n");

    let mut searcher = GreenhouseJobSearcher::new();
    
    // Search parameters
    let keyword = "principal product manager";
    let location = "94555"; // Fremont, CA area
    
    searcher.search_jobs(keyword, location).await?;
    
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
