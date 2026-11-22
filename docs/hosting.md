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

The [Dockerfile](../Dockerfile) runs the latest release. Keep `/data` on a persistent volume; it holds `world.dynw` and `players.txt`.

Any host that runs containers and proxies HTTPS with WebSocket works (for example Railway, Fly.io or a small VPS). Set the service port to the `PORT` variable. Players then join with `https://<your-service-domain>`.

## Files

| File | Contents |
|---|---|
| `world.dynw` | The world, saved every 10 seconds and on shutdown |
| `players.txt` | Last position of each player name |

Back up both files to keep the world.
