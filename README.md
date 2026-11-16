# Skaldsaga

**A story mentor that helps first-time writers turn a rough idea into a complete outline, one question at a time.**

A *skald* was a Norse poet-storyteller, and *saga* means "story" (in modern Icelandic, *skáldsaga* is the everyday word for a novel). Skaldsaga works like an experienced writing mentor. It asks about your story, keeps track of everything you decide, points out contradictions and loose ends, and teaches craft as you go. You stay the author throughout.

> **Early test version.** You can start a new story and pick it back up where you left off. The rest of the conversation, such as working through the middle of your story scene by scene, is still being built. This page describes where Skaldsaga is headed, and says plainly when something isn't ready yet.

## Who it's for

People who have a story in their head but have never written one. You don't need to know anything about story theory, writing terms or programming. Skaldsaga explains every question in plain English and tells you why it matters to your story.

Experienced writers are welcome too. You can skip what you already know.

## What you end up with

A **complete, detailed outline** of your story:

- the people, places and facts of your story world, all consistent with each other
- the shape of your story: who changes, what's at stake, how it ends and why
- every scene in reading order, each with its purpose

"Complete" means more than "finished." The big questions are answered, and the outline holds together:

- Nothing happens before its cause, and nothing contradicts what's already true.
- No character acts on something they haven't learned yet.
- Every hint pays off, and every payoff was set up.
- The ending answers the question the story asked.

## What you'll need

- **Claude Code**, Anthropic's assistant, installed on your computer. Skaldsaga runs inside it. Anthropic's [setup page](https://code.claude.com/docs/en/setup) has the install steps for each kind of computer, and there is a [guide for people who have never used a terminal](https://code.claude.com/docs/en/terminal-guide). For now Skaldsaga works on Linux; Mac and Windows are planned for the first public release.
- **A Claude account that includes Claude Code.** A Pro, Max, Team or Enterprise plan does; the free plan doesn't. You log in the first time you start Claude Code. Skaldsaga uses your Claude account for the conversation; there's nothing else to pay for.
- **Git**, a free tool that keeps a history of your story so you can always go back. You can get it from [git-scm.com/downloads](https://git-scm.com/downloads). Many computers already have it, and Skaldsaga tells you if yours doesn't.

You don't need a GitHub account.

## Getting started

1. **Install Skaldsaga.** In Claude Code, type these two lines, one after the other:

   ```
   /plugin marketplace add mgschiebel/skaldsaga#dist
   /plugin install skaldsaga@skaldsaga
   ```

2. **Open an empty folder for your story** in Claude Code. Use a new, empty folder for each story.
3. **Type `/skaldsaga`.** That's the only command you need to remember. You can type your idea right after it, or just press enter and Skaldsaga will ask what your story is about.

**Getting a newer version.** In Claude Code, type `/plugin marketplace update skaldsaga`, then start a fresh session. If you'd rather it happen on its own, type `/plugin`, choose **Marketplaces**, pick `skaldsaga` and select **Enable auto-update**.

## Writing your story

Skaldsaga asks one question at a time, about things you can picture: a scene, a character, a moment. It works out the bigger structure from your answers. It says back what it understood in its own words, and keeps an answer once you've confirmed it or simply carried on past it. When it's guessing at what you mean, it asks you first. If it gets something wrong, say so and it fixes it before keeping anything.

**Take breaks whenever you like.** Your answers are saved as you go. Next time, open the same folder and type `/skaldsaga` on its own. If you stopped in the middle of something, Skaldsaga gives you a short recap of what you've decided and asks the one question that picks things back up. When a part of your story has run its course, or your answers get short, Skaldsaga stops to say what that part added, offers a few things to look at next (with a suggestion), and offers to save where you are. If you stop at that point, next time it suggests where to start.

**Skaldsaga stays on your story.** If you wander off on a tangent, it reminds you what you were working on, says it is keeping what you said (it can come back later as an idea), and asks where you're heading with it. It might be a new idea or a lost thread; either is fine. A new idea becomes what you work on next, and a lost thread takes you back to where you were.

**You don't have to know the answers yet.** If you're unsure, say so. Skaldsaga offers to throw some ideas around for that part of the story, or you can move on.

**Ideas when you want them.** Ask for some ("give me some ideas for the ending", "what if..."), or take the offer when you're stuck, and Skaldsaga gives you one to three ideas, a line each, built from what you've already decided. If you like one, it first checks whether it would stop anything in your story from fitting and tells you plainly if it would, then it goes in. Ideas you don't use are kept. One may come up again later as a way to answer a question, once at a time, and if you wave one off it never comes back.

**You can change your mind.** If a new answer conflicts with an earlier one, Skaldsaga points it out while the topic is fresh and lets you choose what to keep.

**Your story stays yours.** Skaldsaga keeps your story in plain files in your folder, and it never changes your writing behind your back.

## You'll pick up the craft along the way

Skaldsaga teaches as it goes, with no quizzes. Along the way you'll meet a few common craft terms, each explained in plain words when a question needs it. Before long you'll be using your own names for the people, places and moments in your story instead.

## What's not ready yet

- Carrying on past the basic shape of the story (how it begins, what gets in the way, the worst moment, the ending).
- Bringing in writing you already have, such as drafts and notes. For now, start in an empty folder.
- Later stages: drafting scenes in your own voice, editing, and preparing a finished story for publishing (an e-book, a print book or a web serial).

## Where the ideas came from

Skaldsaga draws on many ideas about how stories work. Its early design was inspired in part by Dramatica, the story theory created by Melanie Anne Phillips and Chris Huntley. Skaldsaga is not affiliated with or endorsed by The Dramatica Co., uses none of its licensed tools, and doesn't ask you to learn any story theory. The goal is a good story, not a theory-correct one. Dramatica® is a registered trademark of its owner.

## For contributors

Skaldsaga is free and open source, and the project is still in active development. If you'd like to help, start with [`CONTRIBUTING.md`](CONTRIBUTING.md), then [`HANDOFF.md`](HANDOFF.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).

## License

Skaldsaga is free software under the [GNU Affero General Public License, version 3 or later](LICENSE), with the additional attribution terms in [`NOTICE`](NOTICE).

In short: you may use, study, change and share it, including commercially, and run it as a service. If you share it or a modified version, or let other people use a modified version over a network, you must give them the source under the same license, keep the copyright and attribution notices, and mark your changes as yours. `NOTICE` and `LICENSE` are the authority, not this summary.
