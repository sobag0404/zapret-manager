# Telegram Relay Worker

This is a deploy-gated, user-owned Cloudflare Worker for the optional Telegram
Desktop relay proof. It is not part of the Zapret Manager installer.

The Worker is not a general proxy. A request selects only a compiled-in
Telegram data center and role. The client cannot provide a hostname, IP, port,
redirect, or HTTP destination.

## Security Gate

1. Review `docs/TELEGRAM_CLOUDFLARE_RELAY_SECURITY.md`.
2. Authenticate with Wrangler:

   ```powershell
   corepack pnpm --dir cloudflare/telegram-relay exec wrangler login
   ```

3. Confirm the account without exposing credentials:

   ```powershell
   corepack pnpm --dir cloudflare/telegram-relay exec wrangler whoami
   ```

4. Choose a new test Worker name and verify that it does not exist in the
   authenticated account. Deployment is intentionally not exposed as a package
   script: the coordinator must record the selected account and unique name,
   inspect existing Worker metadata, and pass that name explicitly to Wrangler.
5. Generate a high-entropy token locally and set it through Wrangler's secret
   prompt. Never put it in this repository, a command argument, URL, or log:

   ```powershell
   $workerName = "<reviewed-unique-test-worker-name>"
   corepack pnpm --dir cloudflare/telegram-relay exec wrangler secret put RELAY_TOKEN --name $workerName
   ```

6. Run the local checks and dry deployment:

   ```powershell
   corepack pnpm --dir cloudflare/telegram-relay check
   corepack pnpm --dir cloudflare/telegram-relay deploy:dry
   ```

7. After the account/name gate, deploy only the separately named test Worker
   by passing the reviewed unique name explicitly:

   ```powershell
   corepack pnpm --dir cloudflare/telegram-relay exec wrangler deploy --name $workerName
   ```

Cloudflare can observe connection metadata and carries an encrypted MTProto
stream. The Worker source contains no logging or analytics binding.
