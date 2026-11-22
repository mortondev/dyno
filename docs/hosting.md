# Hosting a Dyno server

A Dyno server is one process. Players connect with WebSocket, so the server works behind any HTTPS proxy that supports WebSocket.

## On your computer

```sh
dyno server --port 7400 --world ./world/world.dynw --auto-update
```

- Same network: players join with `<your LAN IP>:7400`.
- Internet without port forwarding: `cloudflared tunnel --url http://localhost:7400` prints an `https://…trycloudflare.com` address. Players paste it into *Server address*. The address changes each time the tunnel restarts. The tunnel and server stop when your computer sleeps.
- Internet with port forwarding: forward TCP 7400 to your computer and share your public IP.

## Always on (container)

The [Dockerfile](../Dockerfile) runs the latest release. Keep `/data` on a persistent volume; it holds the world and the files below.

Any host that runs containers and proxies HTTPS with WebSocket works (for example Railway, Fly.io or a small VPS). Set the service port to the `PORT` variable. Players then join with `https://<your-service-domain>`.

## Owner and operators

The first player to join a new world owns it. The owner can use `/gamemode`, `/time`, `/give`, `/summon` and `/reload`. To choose operators yourself, start the server with `--op NAME` (repeat it for more names); then there is no automatic owner.

## Discord

Create a webhook in a Discord channel (Channel settings, Integrations, Webhooks) and start the server with `--webhook <url>`, or set `DYNO_WEBHOOK`. The server posts joins, milestones, deaths and each new day. Keep the URL private: anyone with it can post to the channel.

## Content

Items, recipes, block drops, creatures and milestones are in `content/hearthblock.json`. Run with `--content my.json` to use your own file, and type `/reload` in game to apply changes while people play.

## Files

| File | Contents |
|---|---|
| `world.dynw` | The world, saved every 10 seconds and on shutdown |
| `players.json` | Each player's position, pack, health, food, spawn point and milestones |
| `chests.json` | The contents of every chest |
| `server.json` | The world's owner, the day number, the time of day and recent events |

Back up all four files to keep the world.
