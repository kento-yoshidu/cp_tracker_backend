# API仕様書

競技プログラミングの精進記録を管理するREST APIサーバー。

このドキュメントは実装(`src/`配下)を正として記述している。

## 概要

| 項目 | 内容 |
|------|------|
| フレームワーク | Actix-web 4 |
| データストア | S3(`USE_LOCAL_FILE=1` を設定するとローカルの `problems.json` にフォールバック) |
| 認証 | あり(管理者1名のみ。AWS Cognito + Cookieセッション) |

エンドポイントに `/api` プレフィックスは無い(すべてルート直下、例: `/problems`)。

## デプロイ構成

```
フロントエンド (AWS Amplify)
        ↓
バックエンド (Render)
        ↓
S3バケット: cp-tracker-db
```

バックエンドはステートレスなため、再起動・再デプロイでデータは消えない。

### S3バケット構造

```
cp-tracker-db/
  problems.json    # 問題一覧(メタデータ)のみ。メモ・画像は未実装
```

---

## データモデル

### Problem(JSONに保存するフィールド)

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | String (UUID v4) | 自動生成 |
| `platform` | String | プラットフォーム名(例: `AtCoder`) |
| `url` | String | 問題ページのURL |
| `title` | String | 問題タイトル |
| `tags` | Vec\<String\> | タグ(空配列可) |
| `difficulty` | u16 | 難易度 |
| `ac_count` | u8 | AC回数。新規登録時は`0`、`POST /problems/:id/ac`のたびに+1 |
| `created_at` | Option\<String\> (RFC3339) | 登録日時。新規登録時に自動セット。フィールド追加前の既存データには無いため`None`になりうる |
| `last_solved_at` | Option\<String\> (`yyyyMMdd`) | 最終AC日時。`POST /problems/:id/ac`実行時にセット。`created_at`とは異なりRFC3339ではなく`%Y%m%d`形式 |

### APIレスポンスの形(Problem)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "platform": "AtCoder",
  "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
  "title": "Perfect Ranking",
  "tags": ["greedy", "sorting"],
  "difficulty": 300,
  "ac_count": 2,
  "created_at": "2026-07-15T10:00:00+09:00",
  "last_solved_at": "20260719"
}
```

---

## 認証

管理者(自分)のみがログインでき、書き込み系エンドポイント(`/problems`配下のPOST/PUT/DELETE)はログイン必須。閲覧系(GET)は無認証でアクセス可能。

- ユーザーはAWS Cognito User Poolに管理者1名のみ登録されている(セルフサインアップは無効)
- Cognitoとの通信(`InitiateAuth`)はすべてバックエンドが行う
- ログイン成功時、Cognitoのアクセストークンを`session`という名前のCookie(`HttpOnly`/`Secure`/`SameSite=None`、有効期限1日)にセットする
- 認証必須のエンドポイントは`require_auth`ミドルウェアで保護される。リクエストの`session`CookieをCognitoのJWKSで署名検証し、`token_use=access`かつ`client_id`が一致することを確認する。検証に失敗した場合は`401 Unauthorized`
- 認証必須: `POST /problems`, `PUT /problems/:id`, `DELETE /problems/:id`, `POST /problems/:id/ac`

### ログイン

```
POST /login
```

**リクエストボディ**

```json
{
  "username": "admin",
  "password": "..."
}
```

- Cognitoに対して`USER_PASSWORD_AUTH`フローで認証する
- 成功時は`session`Cookieを発行する(レスポンスボディなし)

**レスポンス** `200 OK` / `401 Unauthorized`(認証失敗)

### ログイン状態確認

```
GET /me
```

`session`Cookieを検証するだけのエンドポイント。

**レスポンス** `200 OK`(有効) / `401 Unauthorized`(未ログイン・検証失敗)

### 環境変数(認証関連)

```
COGNITO_REGION=
COGNITO_USER_POOL_ID=
COGNITO_CLIENT_ID=
COGNITO_CLIENT_SECRET=
```

---

## APIエンドポイント

### 一覧取得

```
GET /problems
```

認証不要。`created_at`の降順(登録が新しい順)にソートして返す。`created_at`を持たない旧データは末尾に回る。

**レスポンス** `200 OK` / `500 Internal Server Error`

```json
[
  { ...Problem },
  { ...Problem }
]
```

---

### 重複チェック

```
GET /problems/check-duplicate?url=...
```

認証不要。末尾の`/`を除去したうえで既存の`url`と完全一致するものがあるか確認する。

**レスポンス** `200 OK`

```json
{ "exists": true }
```

---

### 新規登録

```
POST /problems
```

認証必須。

**リクエストボディ**

```json
{
  "platform": "AtCoder",
  "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
  "title": "Perfect Ranking",
  "tags": ["greedy"],
  "difficulty": 300
}
```

- `id`は自動生成(UUID v4)
- `ac_count`は`0`で初期化
- `created_at`は現在日時(RFC3339)を自動セット
- `last_solved_at`は`null`
- URLの重複チェックはここでは行わない(フロント側で事前に`GET /problems/check-duplicate`を呼ぶ想定)

**レスポンス** `201 Created`(登録後のProblem) / `404 Not Found`(problems.json読み込み失敗) / `500 Internal Server Error`(書き込み失敗)

---

### 更新

```
PUT /problems/:id
```

認証必須。`title` / `url` / `tags` / `difficulty`の4項目を全項目必須で受け取り上書きする(部分更新不可)。`platform` / `ac_count` / `created_at` / `last_solved_at`はこのエンドポイントでは変更されない。

**リクエストボディ**

```json
{
  "title": "Perfect Ranking",
  "url": "https://atcoder.jp/contests/abc123/tasks/abc123_a",
  "tags": ["greedy", "sorting"],
  "difficulty": 300
}
```

**レスポンス** `200 OK`(更新後のProblem) / `404 Not Found` / `500 Internal Server Error`

---

### 削除

```
DELETE /problems/:id
```

認証必須。`problems.json`から該当レコードを削除する。

**レスポンス** `200 OK`(ボディなし) / `404 Not Found` / `500 Internal Server Error`

---

### AC記録

```
POST /problems/:id/ac
```

認証必須。`ac_count`を+1し、`last_solved_at`を現在日時(`%Y%m%d`)にセットする。

**レスポンス** `200 OK`(更新後のProblem) / `404 Not Found` / `500 Internal Server Error`

---

## デバッグ用エンドポイント

API仕様の対象外だが実装に存在するもの。

```
GET /hello   # 疎通確認。"Hello World" を返す
GET /data    # problems.json をS3から生で取得して返す
```

---

## ソースファイル構成

```
backend/
  src/
    main.rs       # サーバー起動・ルーティング
    models.rs     # データ構造体(Problem, 各リクエスト/レスポンス型)
    store.rs      # S3 / ローカルファイル読み書き(problems.json)
    handlers.rs   # /problems 配下のハンドラー
    auth.rs       # Cognito認証(ログイン・JWT検証・認証ミドルウェア)
```
