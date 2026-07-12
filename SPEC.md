# API仕様書

競技プログラミングの精進記録を管理するREST APIサーバー。

## 概要

| 項目 | 内容 |
|------|------|
| フレームワーク | Actix-web 4 |
| データストア | S3（メタデータ・メモ・画像をすべて管理） |
| 認証 | あり（管理者1名のみ。AWS Cognito + Cookieセッション） |

## デプロイ構成

```
フロントエンド (AWS Amplify)
        ↓
バックエンド (Render など)
        ↓
S3バケット（データ永続化）
```

バックエンドはステートレスなため、再起動・再デプロイでデータは消えない。
ローカル開発時は MinIO（S3互換）をDockerで立てることで同じコードが動く。

### S3バケット構造

```
bucket/
  problems.json        # メタデータ一覧
  memos/
    {id}.md            # メモ本文（問題ごとに1ファイル）
  images/              # 将来の画像添付用
    {id}/
      {filename}
```

---

## データモデル

### Problem（JSONに保存するフィールド）

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | String (UUID v4) | 自動生成 |
| `platform` | String | プラットフォーム名（例: `AtCoder`） |
| `problem_id` | String | 問題ID（例: `ABC123_A`） |
| `url` | String | 問題ページのURL |
| `title` | String | 問題タイトル |
| `difficulty` | Option\<u32\> | 難易度・レート |
| `tags` | Vec\<String\> | アルゴリズムタグ（例: `["dp", "bfs"]`） |
| `approach` | String | 解法・実装方法（短文） |
| `ac` | bool | ACしたかどうか |
| `solve_count` | u32 | 解いた回数（新規登録時は1） |
| `review_count` | u32 | 復習した回数（新規登録時は0） |
| `first_solved_at` | String (RFC3339) | 初回解答日時（自動セット） |
| `last_solved_at` | String (RFC3339) | 最終解答日時（自動セット） |

`memo` フィールドはJSONに持たず、S3の `memos/{id}.md` に保存する。

### APIレスポンスの形（Problem）

メモはフロントエンドで別ページ遷移して閲覧するため、問題レスポンスには含めない。

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "platform": "AtCoder",
  "problem_id": "ABC123_A",
  "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
  "title": "PerFect Ranking",
  "difficulty": 300,
  "tags": ["greedy", "sorting"],
  "approach": "ソートして比較",
  "ac": true,
  "solve_count": 2,
  "review_count": 3,
  "first_solved_at": "2024-01-10T10:00:00+09:00",
  "last_solved_at": "2024-03-15T14:30:00+09:00"
}
```

### `problems.json` の構造

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "platform": "AtCoder",
    "problem_id": "ABC123_A",
    "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
    "title": "PerFect Ranking",
    "difficulty": 300,
    "tags": ["greedy", "sorting"],
    "approach": "ソートして比較",
    "ac": true,
    "solve_count": 2,
    "review_count": 3,
    "first_solved_at": "2024-01-10T10:00:00+09:00",
    "last_solved_at": "2024-03-15T14:30:00+09:00"
  }
]
```

### `memos/{id}.md` の例

```markdown
# 思考メモ

## 最初のアプローチ

最初はO(n^2)で全探索を考えたが、制約 n ≤ 2×10^5 を見てO(nlogn)に修正。

## 詰まった点

インデックスのオフセットを1ずれて実装してしまい、WA→ACまで30分かかった。
```

---

## 認証

管理者（自分）のみがログインでき、書き込み系エンドポイント（POST/PUT/DELETE）はログイン必須。閲覧系（GET）は引き続き無認証でアクセス可能。

- ユーザーは AWS Cognito User Pool に管理者1名のみ登録されている（セルフサインアップは無効）
- Cognitoとの通信（`InitiateAuth`）はすべてバックエンドが行い、フロントエンドはCognito SDKを使わない
- ログイン成功時、Cognitoのアクセストークンを `session` という名前の Cookie（`HttpOnly` / `Secure` / `SameSite=None`）にセットする
- 書き込み系エンドポイントは、リクエストの `session` Cookieを Cognito の JWKS で署名検証し、`token_use=access` かつ `client_id` が一致することを確認する。検証に失敗した場合は `401 Unauthorized`
- 現時点で認証必須になっているのは `POST /api/problems`（新規登録）と `POST /api/problems/:id/ac`。今後 PUT/DELETE系を実装する際も同様に保護する想定

### ログイン

```
POST /api/login
```

**リクエストボディ**

```json
{
  "username": "admin",
  "password": "..."
}
```

- Cognitoに対して `USER_PASSWORD_AUTH` フローで認証する
- 成功時は `session` Cookieを発行する（レスポンスボディなし）

**レスポンス** `200 OK` / `401 Unauthorized`（認証失敗）

### 環境変数（認証関連）

```
COGNITO_USER_POOL_ID=
COGNITO_REGION=
COGNITO_CLIENT_ID=
COGNITO_CLIENT_SECRET=
FRONTEND_ORIGIN=   # CORSの許可オリジン（AmplifyのURL）
```

---

## APIエンドポイント

### 一覧取得

```
GET /api/problems
```

一覧取得ではメモ本文は返さない（パフォーマンスのため）。

**クエリパラメータ（任意）**

| パラメータ | 説明 | 例 |
|---|---|---|
| `tag` | タグで絞り込み（完全一致） | `?tag=dp` |

**レスポンス** `200 OK`

```json
[
  { ...Problem },
  { ...Problem }
]
```

---

### 1件取得

```
GET /api/problems/:id
```

メタデータのみ返す（メモは含まない）。

**レスポンス** `200 OK` / `404 Not Found`

---

### メモ取得

```
GET /api/problems/:id/memo
```

S3の `memos/{id}.md` の内容を文字列で返す。ファイルが存在しない場合は空文字を返す。

**レスポンス** `200 OK`

```json
{
  "memo": "# 思考メモ\n\n## 最初のアプローチ\n..."
}
```

---

### 新規登録

```
POST /api/problems
```

**リクエストボディ**

```json
{
  "platform": "AtCoder",
  "problem_id": "ABC123_A",
  "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
  "title": "PerFect Ranking",
  "difficulty": 300,
  "tags": ["greedy"],
  "approach": "ソートして比較",
  "memo": "# 思考メモ\n\n...",
  "ac": true
}
```

- `id` は自動生成（UUID v4）
- `solve_count` は `1` で初期化、`review_count` は `0` で初期化
- `first_solved_at` / `last_solved_at` は現在日時を自動セット
- `memo` が存在すれば S3の `memos/{id}.md` に書き出す

**レスポンス** `201 Created`（登録後のProblemオブジェクト）

---

### 更新（再解答含む）

```
PUT /api/problems/:id
```

**リクエストボディ**（変更したいフィールドのみ）

```json
{
  "approach": "修正した解法",
  "ac": true,
  "increment_solve_count": true,
  "increment_review_count": false
}
```

- `increment_solve_count: true` を渡すと `solve_count` をインクリメントし `last_solved_at` を現在日時に更新する
- `increment_review_count: true` を渡すと `review_count` をインクリメントする

**レスポンス** `200 OK`（更新後のProblemオブジェクト）/ `404 Not Found`

---

### メモのみ更新

```
PUT /api/problems/:id/memo
```

メモを頻繁に編集することを想定した専用エンドポイント。S3の `memos/{id}.md` を上書きする。

**リクエストボディ**

```json
{
  "memo": "# 更新したメモ\n\n..."
}
```

**レスポンス** `200 OK` / `404 Not Found`

---

### 削除

```
DELETE /api/problems/:id
```

`problems.json` からレコードを削除し、S3の `memos/{id}.md` も合わせて削除する。

**レスポンス** `204 No Content` / `404 Not Found`

---

## ソースファイル構成

```
backend/
  src/
    main.rs       # サーバー起動・ルーティング
    models.rs     # データ構造体（Problem, リクエスト/レスポンス型）
    store.rs      # S3読み書き（problems.json・memos/{id}.md）
    handlers.rs   # 各エンドポイントのハンドラー
    auth.rs       # Cognito認証（ログイン・JWT検証・認証ミドルウェア）
frontend/
  ...             # Next.js or React
```

---

## 将来の拡張（低優先度）

### 画像添付

**追加エンドポイント**

```
POST   /api/problems/:id/images   # 画像アップロード → S3に保存しURLを返す
DELETE /api/problems/:id/images/:filename
```

**メモ内での参照方法**

```markdown
![説明](https://bucket.s3.amazonaws.com/images/{id}/{filename})
```

**実装時の考慮点**
- 問題削除時に `images/{id}/` 以下も合わせて削除する
- `aws-sdk-s3` クレートを使用（store.rs と共通）
