# Skaldsaga

**A story mentor that helps first-time writers turn a rough idea into a complete, plot-hole-free outline.**

A *skald* was a Norse poet-storyteller, and *saga* means "story" (in modern Icelandic, *skáldsaga* is the everyday word for a novel). Skaldsaga is an open-source set of skills for [Claude Code](https://code.claude.com) that acts like an experienced writing mentor. It asks you questions about your story, quietly keeps track of everything you decide, catches contradictions and loose ends, and teaches craft as you go.

> **Status: early test build.** You can install it and it will set up a story folder, but the story conversation itself isn't built yet. This README describes how Skaldsaga is meant to work. The design lives in [`HANDOFF.md`](HANDOFF.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Who it's for

People who have a story in their head but have never written one. You don't need to know story theory, writing jargon, or anything about programming. Skaldsaga explains every question in plain English and tells you why it matters to your story.

Experienced writers are welcome too. You can skip what you already know.

## What you get

At the end of the first phase, you have a **complete, detailed outline**:

- the people, places and facts of your story world, all consistent with each other
- the shape of your story: who changes, what's at stake, how it ends and why
- every scene in reading order, each with its purpose and beats

"Complete" means more than "finished." It means the big story questions are answered, every storyline has something happening at every stage, and the plot-hole checks run clean:

1. **Continuity and timeline:** nothing happens before its cause, and nothing contradicts what's already true.
2. **Knowledge:** no character acts on something they haven't learned yet, and no twist arrives before it's earned.
3. **Setups and payoffs:** every hint pays off, and every payoff was set up.
4. **The ending answers the story's question.**
5. **Cause and effect:** each scene follows from what came before.

## Before you start

You'll need:

- **Claude Code**, Anthropic's coding assistant, installed on your computer. Skaldsaga runs inside it. Test builds run on Linux; Mac and Windows come with the first public release.
- **A Claude account that includes Claude Code**, such as a paid Claude plan or a Claude Console (API) account. Skaldsaga uses your Claude account for the conversation; there's nothing else to pay for.
- **Git**, the free tool Skaldsaga uses to keep a history of your story so you can always go back. Many computers already have it; Skaldsaga tells you if it's missing.

You don't need a GitHub account: Skaldsaga installs from its public page on GitHub without signing in.

## How it works

1. **Install** Skaldsaga as a Claude Code plugin. In Claude Code, type `/plugin marketplace add mgschiebel/skaldsaga#dist`, then `/plugin install skaldsaga@skaldsaga`.
2. **Open a folder** for your story in Claude Code and type `/skaldsaga`. That's the only command you need to remember.
3. **Talk about your story.** Skaldsaga asks about concrete things, such as a scene you can picture or a character you know, and works out the bigger structure from your answers. It always checks with you ("It sounds like your hero ends up changing. Is that right?").
4. **Take breaks whenever you like.** Everything is saved after each answer you confirm. When it's time for a fresh start, Skaldsaga tells you. Next time, open the folder and it greets you with a recap of where you left off.

### Things it handles for you

- **You don't have to know the answers yet.** Unsure answers are kept as "to be decided" and settled later, when the story makes them clearer. If you're stuck, Skaldsaga offers "what if" options built from your own ideas.
- **You can change your mind.** If a new answer conflicts with an earlier one, Skaldsaga points it out while the topic is still fresh and lets you choose what to keep.
- **You can bring writing you already have.** Point it at an existing folder of drafts and notes. It looks before touching anything, recommends keeping your originals safe in an archive, and pulls the story facts out of your drafts for you to approve. It never changes your writing behind your back.
- **It isn't tied to novels.** Your story is kept as one sequence of scenes. Chapters, a play's acts, or a web serial's weekly updates are just different ways to group those scenes later.

## What it doesn't do (yet)

The first phase stops at the outline. Later phases are planned to help with drafting scenes in your own voice, editing, and preparing editions for publishing: an e-book, a print book, or a web serial. You stay the author throughout.

Skaldsaga draws on many story-structure ideas. Its early design was inspired in part by Dramatica, the theory of story created by Melanie Anne Phillips and Chris Huntley. Skaldsaga is not affiliated with or endorsed by The Dramatica Co., uses none of its licensed tools, and doesn't ask you to learn any story theory. The goal is a good story, not a theory-correct one. Dramatica® is a registered trademark of its owner. More about where the ideas came from is in [`docs/background.md`](docs/background.md).

## For contributors

The project is in its planning phase. Start with:

- [`HANDOFF.md`](HANDOFF.md): the index to every decision so far (status, what's next, and a map of the reference files in [`docs/handoff/`](docs/handoff/) that hold the reasoning and open questions)
- [`ARCHITECTURE.md`](ARCHITECTURE.md): a one-page sketch of the components and how they fit together
- [`docs/background.md`](docs/background.md): where the ideas came from, research notes and attribution (kept out of the build documents on purpose)

## License

Skaldsaga is free software under the [GNU Affero General Public License, version 3 or later](LICENSE), with the additional attribution terms in [`NOTICE`](NOTICE).

In short: you may use, study, change and share it, including commercially, and run it as a service. If you share it or a modified version, or let other people use a modified version over a network, you must give them the source under the same license, keep the copyright and attribution notices, and mark your changes as yours. `NOTICE` and `LICENSE` are the authority, not this summary.
