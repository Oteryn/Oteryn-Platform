# GAP-56 · Moderated article discussions

The Platform implementation uses article comments on published News posts. Each
news article is a thread shared by its published Polish and English versions.
Readers reach it from Community → News → the article → Article discussion.
Drafts, scheduled articles before publication, changelog entries, and unavailable
translations have no discussion access. If the article is taken offline or
deleted, its replies stop appearing publicly. A new article that reuses the
slug opens its own discussion.

| State | Public | Author | Staff |
| --- | --- | --- | --- |
| Awaiting review | — | Read and edit | Publish or reject |
| Published | Read | Read or edit and return to review | Hide |
| Rejected | — | Read or revise and resubmit | Read the decision history |
| Hidden | — | Read | Read the decision history |

Posting requires an active, verified Platform account without an effective
suspension. Replies are plain text, with a 2,000-character limit and a rolling
five-reply hourly limit per author. Public replies show their creation time and
text. Author identity and moderation reasons stay private.

## Staff workflow

Open News → Discussion moderation in the staff panel. The queue defaults to
Awaiting review and offers filters for each other state. Staff must hold
`admin.access` and `cms.news.manage` and satisfy the existing staff MFA
guard. Another editor reviews an editor's own reply. The detail view shows
the reply, article availability, current state and moderation history.
Each decision requires an internal reason and records the editor, resulting
state, revision and timestamp in the existing append-only audit log. Reasons
should omit personal data. The operator can reject pending replies or hide
published replies after the source article goes offline.

## Game dependencies

None. Discussion uses Platform publication, account and moderation state.
