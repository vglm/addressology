# Addressology backend API

This document describes the HTTP endpoints implemented by the Rust backend in this repository.

## Conventions

- Base API scope: `/api`
- Authentication for protected endpoints is session-based via the `web-portal-session` cookie.
- Most request and response bodies use JSON with `camelCase` field names.
- Unless noted otherwise, timestamps are returned as database/serde datetime values and query `since` filters expect an RFC 3339 timestamp.
- Protected endpoints return `401 Unauthorized` when the session is missing.

## Common response shapes

### User object

Returned by login/session endpoints. Serialized fields:

- `uid`
- `email`
- `createdDate`
- `lastPassChange`
- `allowPassLogin`
- `allowGoogleLogin`
- `tokens`

Sensitive fields such as password hashes and reset tokens are not serialized.

### Contract object

Returned by contract endpoints:

- `contractId`
- `userId`
- `created`
- `address`
- `network`
- `data`
- `tx`
- `deployStatus` (`None`, `Requested`, `TxSent`, `Failed`, `Succeeded`)
- `deployRequested`
- `deploySent`
- `deployed`

## Non-API backend routes

### `GET /`

Permanent redirect to `/dashboard/`.

### `GET /dashboard`

Permanent redirect to `/dashboard/`.

### `GET /dashboard/{path...}`

Serves the frontend dashboard assets or proxies them in proxy mode.

### `POST /service/update`

Uploads a replacement binary/update file through multipart form data.

- Header: `Update-Secret`
- Body: multipart upload
- Returns `200 OK` with `File uploaded successfully` on success
- Returns `401 Unauthorized` when updates are disabled or the secret is invalid

## Authentication and session endpoints

### `GET /api/auth/login/google`

Starts Google OAuth login.

- No request body
- Creates OAuth state in the database
- Responds with a temporary redirect to Google authentication

### `GET /api/auth/callback/google`

Google OAuth callback endpoint.

- Query params:
  - `code`
  - `state`
- Validates the stored OAuth state record, exchanges the code, verifies the Google token, loads the user, and stores the session
- Redirects to `/dashboard/` on success
- Returns `400` for missing/invalid callback parameters
- Returns `401` when the email is not allowed

### `POST /api/login`

Password login.

- Auth: public
- JSON body:
  - `email`
  - `password`
- Returns the user object on success
- Returns `400` if already logged in as another user
- Returns `401` for disallowed email or invalid credentials

### `GET /api/session/check`

Ensures a per-session check token exists and returns it.

- Auth: public
- Returns a plain string token stored in session under `check`

### `GET /api/is_login`
### `POST /api/is_login`

Checks whether the current session is authenticated.

- Auth: public
- Returns the current user object when logged in
- Returns `401` with `Not logged in` otherwise

### `POST /api/logout`

Clears the current session login.

- Auth: public
- Returns `Logged out` when a session user existed
- Returns `Not logged in` when there was nothing to clear

### `POST /api/reset_pass`

Starts password reset for an allowed email.

- Auth: public
- JSON body:
  - `email`
- If account creation is allowed and the user does not exist, a new user record is created first
- Generates and stores a reset token, then sends an email containing a dashboard reset link
- Returns `Reset link sent` on success
- Returns:
  - `401` when the email is not allowed
  - `400` when a reset token was already sent within the last minute
  - `500` for database/storage failures

### `POST /api/set_pass`

Completes password reset with a token.

- Auth: public
- JSON body:
  - `email`
  - `token`
  - `newPassword`
- Verifies allowed email, reset token, token age, and password quality
- Updates the stored password and logs the user out of the current session
- Returns `Password changed successfully` on success
- Returns:
  - `401` for disallowed email or missing user
  - `400` for invalid/expired token or weak password

### `POST /api/change_pass`

Changes the current user password.

- Auth: required
- JSON body:
  - `email`
  - `oldPassword`
  - `newPassword`
- Verifies the old password and password quality, updates the password, and logs the user out
- Returns `Password changed successfully` on success
- Returns:
  - `401` when not logged in, user lookup fails, or old password is invalid
  - `400` for weak password

### `GET /api/greet`

Simple backend health/info endpoint.

- Auth: public
- Returns JSON with:
  - `message`
  - `domain`
  - `version`

## User/token endpoint

### `GET /api/user/tokens`

Returns the latest token balance for the logged-in user.

- Auth: required
- Response:
  - `uid`
  - `email`
  - `tokens`

## Fancy address endpoints

### `GET /api/fancy/score/{address}`

Scores an Ethereum address and optionally enriches the response with mined metadata.

- Auth: optional
- Path param: `address`
- If logged in and the address exists in storage, the response also includes mining/provider details
- Response fields:
  - `score`
  - `price`
  - `minerInfo`
  - `mined`
  - `factory`
  - `salt`
  - `publicKeyBase`
- Returns `400` when the address format is invalid

### `GET /api/fancy/categories`

Returns the list of scoring categories used by the fancy-address scorer.

### `GET /api/fancy/random`

Returns a random non-reserved fancy address from the top scored entries.

- Auth: public
- Query params:
  - `category` (optional, use `all` to disable category filtering)

### `GET /api/fancy/total_hash`

Estimates total hashing work for mined leading-zero addresses.

- Auth: public
- Query params:
  - `since` (optional RFC 3339 datetime)
  - `public_key_base` (optional)
  - `provider_id` (optional)
- Response:
  - `eventDifficulty`
  - `numberOfEvents`
  - `estimatedWorkTH`

### `GET /api/fancy/list`

Lists fancy addresses with filtering.

- Auth: optional
- Query params:
  - `limit` (optional integer, default `100`)
  - `public_key_base` (optional; `all` disables filtering, omitted means only entries without public key base)
  - `category` (optional; `all` disables category filtering)
  - `provider_id` (optional)
  - `free` (optional: `free`, `reserved`, `all`, `mine`)
  - `order` (optional: `score` or `created`, default `score`)
  - `since` (optional RFC 3339 datetime)
- Returns a list of stored fancy-address records
- `free=mine` requires a logged-in user and otherwise returns `401`

### `GET /api/fancy/mylist`

Lists fancy addresses owned/reserved by the logged-in user, enriched with assigned contracts.

- Auth: required
- Query params:
  - `unassigned_only` (optional boolean)
- Response fields per item:
  - address data and score metadata
  - provider information
  - `assignedContracts`

### `POST /api/fancy/new_many`
### `POST /api/fancy/new_many2`

Bulk ingestion endpoint for mined fancy addresses tied to a job.

- Auth: public
- JSON body:
  - `data`: array of entries with `salt`, `factory`, optional `address`
  - `extra`:
    - `jobId`
    - `reportedHashes`
    - `reportedCost`
- Behavior:
  - Accepts either factory addresses or public keys in the `factory` field
  - Computes the address, rejects mismatches/duplicates/low-score entries
  - Updates the referenced mining job counters
- Response:
  - `totalScore`
  - `entriesRejected`
  - `entriesParseError`

### `POST /api/fancy/buy/{address}`

Purchases an unowned fancy address using the logged-in user token balance.

- Auth: required
- Path param: `address`
- Returns `200 OK` with an empty body on success
- Returns:
  - `400` if the address is already owned or the user has insufficient funds
  - `404` if the address is unknown

### `POST /api/fancy/deploy/{contract_id}`

Marks a contract for deployment.

- Auth: required
- Path param: `contract_id`
- If the contract is still in `None` state, updates it to `Requested`
- Returns the updated contract object on success
- Returns status text when deployment was already requested/sent/finished
- Returns `404` when the contract does not exist for the user

### `GET /api/public_key_base/list`

Returns the public key bases associated with the logged-in user.

- Auth: required

## Job endpoints

### `POST /api/job/new`

Creates a new mining job and upserts miner identity information.

- Auth: public
- JSON body:
  - `miner`
    - `provNodeId`
    - `provRewardAddr`
    - `provName`
    - `provExtraInfo`
  - `cruncherVer`
  - `requestorId`
- At least one miner field must be present
- Returns the created job with embedded miner info
- Returns `400` for empty miner info or invalid `requestorId`

### `POST /api/job/finish/{job_id}`

Marks a mining job as finished and returns the refreshed job record.

- Auth: public
- Path param: `job_id` (UUID)
- Returns a `JobWithMinerApi` JSON object
- Returns `400` when `job_id` is not a valid UUID

### `GET /api/job/list`

Lists mining jobs.

- Auth: optional
- Query params:
  - `limit` (optional integer, default `100`)
  - `order` (must be `created`)
  - `status` (`all`, `finished`, `active`; default `all`)
  - `since` (optional RFC 3339 datetime)
  - `requestor_id` (optional Ethereum address)
- Returns a list of job records
- Returns `400` for unsupported query values

## Contract endpoints

### `POST /api/contract/compile`

Compiles Solidity sources with solc `0.8.28`.

- Auth: public
- JSON body:
  - `sources`: object mapping file names to source code strings
- Returns the raw compiler output JSON from `compile_solc`

### `GET /api/contract/{contract_id}`

Fetches one contract belonging to the logged-in user.

- Auth: required
- Path param: `contract_id`
- Returns the contract object when found
- Returns JSON `null` when the database lookup succeeds but no matching contract exists
- Returns `500` on lookup failure

### `POST /api/contract/new`

Creates a new contract record for the logged-in user.

- Auth: required
- JSON body:
  - `address` (optional)
  - `network`
  - `data`
- New records are created with deploy status `None`
- Returns the created contract object

### `POST /api/contract/{contract_id}`

Updates an existing contract owned by the logged-in user.

- Auth: required
- Route includes `contract_id`, but the handler actually uses `contractId` from the JSON body to load and update the record
- JSON body: full contract object
- Rejected only when the stored contract already has an `address` and `deployStatus` is not `None`
- Returns:
  - updated contract object on success
  - `404` if the contract does not exist
  - `400` if deployment has already started

### `GET /api/contracts/list`

Lists all contracts belonging to the logged-in user.

- Auth: required
- Returns an array of contract objects

### `GET /api/contracts/assignments`

Lists contract-to-address assignments for the logged-in user.

- Auth: required
- Returns an array of contract-address assignment objects

### `POST /api/contract/{contract_id}/delete`

Deletes a contract owned by the logged-in user.

- Auth: required
- Path param: `contract_id`
- Allowed only while `deployStatus` is `None`
- Returns empty `200 OK` on success
- Returns:
  - `404` if the contract does not exist
  - `400` if deployment has already started
