# Greenhouse Job Search Architecture

## Overview
This is a Rust-based CLI tool that searches for jobs across multiple companies using Greenhouse.io's API. The system works by first discovering companies that use Greenhouse for their job postings, then querying each company's individual API for job listings.

## Architecture Components

### 1. Data Structures

#### Core Job Models
- **`Job`**: Represents a job posting from Greenhouse API
  - Fields: id, title, updated_at, location, absolute_url, departments
- **`JobLocation`**: Location information for jobs
- **`Department`**: Company department information
- **`JobsResponse`**: API response wrapper containing job arrays
- **`JobResult`**: Simplified job representation for display

#### Main System Component
- **`GreenhouseJobSearcher`**: Central orchestrator containing:
  - HTTP client with timeout and user-agent configuration
  - Set of discovered board tokens (company identifiers)

### 2. Company Discovery System

The system uses a two-tier approach to find Greenhouse-enabled companies:

#### Primary Method: Google Search Discovery
- Searches Google for `site:boards.greenhouse.io` to find active boards
- Extracts board tokens from discovered URLs
- Parses HTML using the `scraper` crate to find Greenhouse board links

#### Fallback Method: Known Board Tokens
- Maintains a curated list of verified company board tokens
- Includes major tech companies: Stripe, Uber, Airbnb, Shopify, etc.
- Used when Google search fails or returns no results

### 3. Job Search Engine

#### API Integration
- Queries Greenhouse's public API: `https://boards-api.greenhouse.io/v1/boards/{token}/jobs`
- Uses `content=true` parameter to get detailed job information
- **Concurrent Processing**: All companies searched simultaneously using `tokio::spawn`
- **Randomized Rate Limiting**: 0-200ms random delays per request to avoid API overload

#### Matching Algorithm
- **Keyword Matching**: Flexible multi-word matching with synonyms
  - Supports variations (e.g., "principal" matches "senior", "staff", "lead")
- **Location Matching**: Broad location filtering including:
  - Exact location matches
  - Remote work options
  - Bay Area variations (SF, Silicon Valley, CA, etc.)

#### Concurrency Architecture
- **Static Method Pattern**: `search_jobs_for_board_static()` for thread-safe execution
- **Task Spawning**: Each company search runs as independent tokio task
- **Thread Safety**: Uses `String` errors instead of `Box<dyn Error>` for `Send` compatibility
- **Progress Aggregation**: Collects results from all concurrent tasks

### 4. Data Flow

```
1. Initialize GreenhouseJobSearcher
2. Discover board tokens:
   - Try Google search for Greenhouse boards
   - Fallback to known tokens if needed
3. Spawn concurrent tasks for all board tokens:
   - Each task queries Greenhouse API independently
   - Random delays prevent API rate limit violations
   - Filter jobs by keyword and location in parallel
4. Collect results from all completed tasks
5. Display aggregated results with completion tracking
```

### 5. Error Handling & Resilience

- **Network Failures**: Graceful handling of API timeouts and connection errors
- **Invalid Responses**: JSON parsing errors handled without stopping search
- **Concurrent Error Isolation**: Failed tasks don't affect other concurrent searches
- **Rate Limiting**: Randomized delays (0-200ms) to respect API limits
- **Debug Information**: Randomized logging to avoid spam while providing insights

### 6. Dependencies

- **`reqwest`**: HTTP client for API calls and web scraping
- **`tokio`**: Async runtime for concurrent operations
- **`serde`**: JSON serialization/deserialization
- **`scraper`**: HTML parsing for Google search results
- **`urlencoding`**: URL parameter encoding
- **`rand`**: Random sampling for debug output

## Key Features

1. **Scalable Company Discovery**: Automatically finds new companies using Greenhouse
2. **Concurrent Search Processing**: Simultaneous API queries across all companies for faster results
3. **Flexible Search**: Supports synonym matching and broad location filtering  
4. **Intelligent Rate Limiting**: Randomized delays prevent API overload while maximizing throughput
5. **Resilient Error Handling**: Concurrent error isolation ensures failed companies don't stop the search
6. **Real-time Progress Tracking**: Shows completion status across all concurrent operations

## Usage Pattern

The tool is designed for job seekers who want to search across multiple companies simultaneously. It targets a specific job title ("principal product manager") and location ("94555" - Fremont, CA area) but can be easily modified for different search criteria.