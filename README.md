# Bolt the Bird

An AI that drafts Magic: The Gathering, builds its deck, and plays it, trained from scratch on
real Arena data from [17lands](https://www.17lands.com/public_datasets).

**Status:** just started. See [PLAN.md](PLAN.md).

1. **Draft:** a transformer that picks cards the way strong human drafters do
2. **Build:** turns a drafted pool into a 40-card deck, scored by a win-rate model trained on real games
3. **Play:** a Rust rules engine and a reinforcement-learning agent that plays the deck

Set: Foundations (FDN). Data: 17lands public datasets (CC BY 4.0).

*Bolt the Bird is unofficial Fan Content permitted under the Fan Content Policy. Not
approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the
Coast. ©Wizards of the Coast LLC.*
