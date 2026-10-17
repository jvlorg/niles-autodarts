# Niles Autodarts

A planned standalone [Niles](https://github.com/MarkNygaard/niles) add-on for dartboard control and spoken score announcements.

**Status: planning only. No implementation, installable package or release yet.**

## Version 1: play darts and hear the score

- Add the Autodarts dartboard as a Niles device and assign it to a room.
- Say “Niles, turn on the dartboard” to start Board Manager detection; support stopping it too and show the actual running state.
- Select which satellites in that room announce scores.
- Use Niles' Piper speech path to speak scores through those selected satellites.
- Enable or disable score announcements.

Starting detection is not physically powering the hardware, and is not starting a game. Game selection remains in Autodarts for V1.

Whether announcements cover each dart, the turn total, or both is still undecided. Authoritative score data and the host's targeted speech API must be verified before implementation.

## Later versions

- **V2:** show a live scoreboard on an LG TV, coordinating power and app/page selection through the LG TV add-on. The display transport is not decided; a web scoreboard may be preferable to a video stream.
- **V3 (tentative):** select and start games from Niles, including the necessary player and game settings.

See [the canonical project plan](docs/plan.md) for scope, acceptance criteria and open questions.

## Development reference

The existing Smart Hub Autodarts module provides local Board Manager protocol knowledge: HTTP start/stop/reset commands, polling, WebSocket events and throw data. It is a reference, not a drop-in Niles implementation. The Niles Withings add-on provides experience with standalone add-on packaging and host lifecycle integration.

No provider credentials, household captures or live configuration belong in this repository.
