# Skaldsaga: GitHub settings to change (security review, October 20, 2026)

Do these on a computer, or a phone browser in desktop mode. The GitHub mobile app can't edit most of these settings. Each step names the security requirement (SR-xx) it closes, from `docs/security/2026-10-20-review.md`.

Repository settings live at: https://github.com/mgschiebel/skaldsaga/settings

Recommended order: 1 → 2 → 3 → 4, then the optional ones. Merge pull request #21 whenever you like; none of these depend on it.

---

## 1. Turn on "Main protect" for the `main` branch (SR-50, most important)

The ruleset exists but its enforcement is **Disabled**, so `main` is unprotected today.

1. Open **Settings → Rules → Rulesets** and select **Main protect**.
2. **Enforcement status:** change to **Active**.
3. **Bypass list:** remove **Repository admin** and **Maintain**.
   - Both are set to "Always", which means you skip every rule.
   - You don't need a bypass: with zero required approvals you can still merge your own pull requests.
   - For an emergency, switch the ruleset to Disabled temporarily.
4. **Target branches:** leave **Default branch**.
5. **Rules:**
   - Keep ticked: **Restrict deletions**, **Block force pushes**, **Require linear history**.
   - **Untick "Restrict updates".** With the bypass removed, this rule would stop anyone (you included) from merging into `main`.
   - **Tick "Require a pull request before merging".**
     - Required approvals: **0** (GitHub doesn't let you approve your own pull request).
     - Allowed merge methods: **Squash** only. That matches how you merge now and keeps history linear.
   - **Tick "Require status checks to pass".**
     - Click **Add checks** and add **Format, lint, test (Linux x64)**.
     - After pull request #21 is merged, also add **Privacy checks** (it appears in the list once it has run once).
6. Click **Save changes**.
7. Tell a Claude session "check the Main protect ruleset", and it can confirm the setup through the API.

## 2. Keep your email private in new commits (SR-02)

1. Open your account settings (not the repository's): **profile picture → Settings → Emails** (https://github.com/settings/emails).
2. Tick **Keep my email addresses private**.
3. Tick **Block command line pushes that expose my email**.
4. Note the no-reply address shown there. Yours is `mgschiebel@users.noreply.github.com`.
5. On any computer where you commit, run:

       git config --global user.email "mgschiebel@users.noreply.github.com"
       git config --global user.name "Mike Schiebel"

## 3. Secret scanning extras and Dependabot (SR-04, SR-43)

Secret scanning and push protection are already on. Add the rest:

1. Open **Settings → Advanced Security** (named **Code security** on some accounts).
2. Under **Secret Protection**, turn on **Scan for non-provider patterns** and **Validity checks**.
3. Under **Dependabot**, turn on **Dependabot alerts** and **Dependabot security updates**.
4. On the same page, turn on **Private vulnerability reporting**, so people can report security problems to you privately (SR-51).
5. On the same page, under **Code scanning**, leave **CodeQL analysis** on **Advanced** (or off). `codeql.yml` is the advanced setup, and GitHub refuses its results while "default setup" is on. Once the baselining pull request is merged, the ruleset's "Code scanning results" rule is set to block on every severity, and the new jobs (Dependency policy, Workflow security, CodeQL) are made required (SR-50).

## 4. Tidy merged branches (SR-53)

1. Open **Settings → General**, scroll to **Pull Requests**.
2. Tick **Automatically delete head branches**.
3. Optional: delete the old `docs/meter-test-pause` branch if you no longer need it (**Code → Branches**, trash icon).

---

## Optional, when you have a few more minutes

### 5. Protect the `dist` branch (part of SR-50)

1. **Settings → Rules → Rulesets → New ruleset → New branch ruleset.**
2. Name: **dist protect**. Enforcement: **Active**. No bypass list.
3. **Target branches → Add target → Include by pattern:** `dist`.
4. Rules: tick **Restrict deletions** and **Block force pushes** only.
   - Don't tick "Restrict updates" or "Require a pull request": the Release workflow adds commits to `dist` directly.
5. **Create.**

### 6. Protect release tags (part of SR-45)

1. **Settings → Rules → Rulesets → New ruleset → New tag ruleset.**
2. Name: **release tags**. Enforcement: **Active**.
3. **Target tags → Include by pattern:** `v*`.
4. Rules: tick **Restrict deletions** and **Restrict updates**.
   - Leave **Restrict creations** unticked: the Release workflow creates each tag itself.
5. **Create.**

### 7. Claude session sharing (not GitHub, but related to SR-03)

Your existing session links stay as they are; this just makes an accidental share safer.

1. On claude.ai, open **Settings → Claude Code → Sharing settings** (https://claude.ai/settings/claude-code).
2. Turn on **require recipients to have repository access**, and choose whether to **hide your name** on shared sessions.
