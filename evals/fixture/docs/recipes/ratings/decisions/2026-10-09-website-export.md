---
id: dc-06dcc4d
node: fd-0da3f4
title: "Ratings come from the website export"
status: accepted
topic: ratings-source
decided_by: priya
date: 2026-10-09
---

**Decision.** The cookbook reads ratings from the website's nightly export in `data/ratings.json`. Readers can't rate from the command line.

**Why.** The website already handles accounts and abuse, and the CLI has no accounts.

**Rejected.** A `cookbook rate` command that writes to a local file.
