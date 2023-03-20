# Reward campaign helper
Helper script to create and add crowdloan contrbuter in datahighway chain

## How to run
- Create a `input.json` file. Example: [`input.json`](res/input.json)
- Download the json list of all crowdloan contributer and put it in same directory as `input.json`. Example: [`contributers.json`](res/contributers.json)
- Download and keep the binary in same directory as above two file
- Make a key file with the seed prashe from which the extrinsic will be signed
Example:
*signer.key*
```
wait sure dignity lamp surround mammal power obey beach suggest useless cabin
```
- Run the script by passing `input.json` as argument and `signer.key` as environment variable `SIGNER_KEY`
Example:
```
SIGNER_KEY="signer.key" ./reward-campaign-helper ./input.json | tee run.log
```

## Inspecting
1) Open up the polkadotjs explorer and follow up the events
2) Logs are also printed after each step in binary
3) Failed call to add an extrinsic will be reported to log and will be skipped to add another contributer

**Note: Script add one contributer per block to ensure error checking, so expected time for script to be executed is at least `BLOCK_PER_SECOND * ( NO_OF_CONTRIBUTERS + 1)`

## Post-script steps
1) After script have finished running, ensure that all contributed are added ( making sure no failed report in console log. ) if some failed contributer exists, add them manually
2) Lock the campaign to finalize and let user claim reward