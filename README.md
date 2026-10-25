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

- **Claude Code**, Anthropic's assistant, installed on your computer. Skaldsaga runs inside it. For now it works on Linux; Mac and Windows are planned for the first public release.
- **A Claude account that includes Claude Code**, such as a paid Claude plan. Skaldsaga uses your Claude account for the conversation; there's nothing else to pay for.
- **Git**, a free tool that keeps a history of your story so you can always go back. Many computers already have it, and Skaldsaga tells you if yours doesn't.

You don't need a GitHub account.

## Getting started

1. **Install Skaldsaga.** In Claude Code, type these two lines, one after the other:

   ```
   /plugin marketplace add mgschiebel/skaldsaga#dist
   /plugin install skaldsaga@skaldsaga
   ```

2. **Open an empty folder for your story** in Claude Code. Use a new, empty folder for each story.
3. **Type `/skaldsaga`.** That's the only command you need to remember. You can type your idea right after it, or just press enter and Skaldsaga will ask what your story is about.

## Writing your story

Skaldsaga asks one question at a time, about things you can picture: a scene, a character, a moment. It works out the bigger structure from your answers, and always checks with you before it keeps anything ("It sounds like your hero ends up changing. Is that right?"). Nothing goes into your story until you say yes.

**Take breaks whenever you like.** Your answers are saved as you confirm them. Next time, open the same folder and type `/skaldsaga` on its own. If you stopped in the middle of something, Skaldsaga gives you a short recap of what you've decided and asks the one question that picks things back up.

**You don't have to know the answers yet.** If you're unsure, say so. Skaldsaga keeps it as "to be decided" and comes back when the story makes it clearer. If you're stuck, it can offer "what if" ideas built from your own.

**You can change your mind.** If a new answer conflicts with an earlier one, Skaldsaga points it out while the topic is fresh and lets you choose what to keep.

**Your story stays yours.** Skaldsaga keeps your story in plain files in your folder, and it never changes your writing behind your back.

## What's not ready yet

- Carrying on past the basic shape of the story (how it begins, what gets in the way, the worst moment, the ending).
- Bringing in writing you already have, such as drafts and notes. For now, start in an empty folder.
- Later stages: drafting scenes in your own voice, editing, and preparing a finished story for publishing (an e-book, a print book or a web serial).

## Where the ideas came from

Skaldsaga draws on many ideas about how stories work. Its early design was inspired in part by Dramatica, the story theory created by Melanie Anne Phillips and Chris Huntley. Skaldsaga is not affiliated with or endorsed by The Dramatica Co., uses none of its licensed tools, and doesn't ask you to learn any story theory. The goal is a good story, not a theory-correct one. Dramatica® is a registered trademark of its owner.

## For contributors

Skaldsaga is free and open source, and the project is still in active development. If you'd like to help, start here:

- [`CONTRIBUTING.md`](CONTRIBUTING.md): the rules for contributions, including licensing
- [`HANDOFF.md`](HANDOFF.md): where the project stands, what's next, and a guide to the design documents in [`docs/`](docs/)
- [`ARCHITECTURE.md`](ARCHITECTURE.md): a one-page sketch of how the pieces fit together
- [`docs/background.md`](docs/background.md): where the ideas came from, with research notes and attribution

## License

Skaldsaga is free software under the [GNU Affero General Public License, version 3 or later](LICENSE), with the additional attribution terms in [`NOTICE`](NOTICE).

In short: you may use, study, change and share it, including commercially, and run it as a service. If you share it or a modified version, or let other people use a modified version over a network, you must give them the source under the same license, keep the copyright and attribution notices, and mark your changes as yours. `NOTICE` and `LICENSE` are the authority, not this summary.
