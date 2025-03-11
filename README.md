my doorlock project

this has a hardware component.
i'm putting it on github so i have cross machine access with version control,
i don't expect it to be useful to anyone, but I will eventually document my circuitry in case
I want to replicate it later on in life (way more fun than buying shit)

TODO:

implement auto lock
- new sensor maybe? or just a timing mechanism.
- I'm thinking like, if the sensor senses the door is shut, it relocks, but this is tricky (electromagnet? sonar?)
- could also just trigger a wait queue on every EnsureUnlock instruction, so it EnsureLocks like 1 minute later
