# dstn.to API

There is probably a lot of better ways to do many of the things I've done here, but this is one of the first things I've done in rust, make suggestions if you see anything done weirdly!

Built with actix-web, makes use of prisma to handle queries to a postgresql database and uses valkey/redis for storage of tokens and caching data.

## Used for

- Spotify History and Now Playing API / Provides realtime queue for [gateway](https://github.com/dustinrouillard/dustin-gateway)
- File and Screenshot Uploads (Multipart uploads to an s3 bucket)
- Github pinned repositories
- Blog System for [Personal Site](https://github.com/dustinrouillard/personal-site)
- Local weather (This just proxies my [weather worker](https://github.com/dustinrouillard/weather-worker))
- Analytics tracking (commands per day, etc)
- Prometheus metrics (API route and process metrics)