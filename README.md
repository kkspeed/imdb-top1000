# imdb-top1000
A simple toy crawler that craws IMDB top 1000 movies.

## Running the Crawler 

1. Install Rust and Cargo, preferrably using [rustup](https://www.rustup.rs/) and
   select *nightly* toolchain.

2. Run <tt>cargo run</tt>

Then the crawler will first crawl the page by spawning 8 threads. When it's done,
it will spawn a web server at <tt>localhost:8000</tt>. Send request to
<tt>http://localhost:8000/your term</tt> to see JSON-serialized results.

## Possible Future Improvement
1. Use a "professional" in-memory DB (e.g. redis) instead of an adhoc hashtable to
   maintain results.
2. Separate crawling and server executable. Both could talk with the DB backend.
