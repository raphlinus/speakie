# speakie

A library for playback of TMS5220 LPC speech.

It was inspired by the Talkie library, plus study of the [MAME] sources, in particular [devices/sound/tms5220.cpp]. The encoded speech is compatible with [Buzzer Studio].

It is `#[no_std]`, does not allocate, and should run well on extremely resource constrained devices.

One feature of the library is interpolation, which improves smoothness considerably. The goal is to produce high quality sound using modest resources, as opposed to the goal in MAME of bit-accurate emulation. Another major source was [TSP50C0X/1X Family Speech Synthesizer Design Manual], which among other things is precedent for linear interpolation of the coefficients. This chip was a microcontroller that played back TMS5220 encoded speech.

The TMS5220 is one of the later chips in the series of [Texas Instruments LPC Speech Chips]. It was used in the [Echo II] speech synthesis board for the Apple 2 and the PCjr speech, among other things. These chips have different coefficient tables, so bitstreams encoded for other chips may sound somewhat off or be unplayable. See [Chipspeech diary, part 2] for more information about the speech chip variants.


The provided demo app can accept the hex LPC data as a command line argument, as stdin, or read from a file. It outputs a WAV file.

```
cargo run --example demo "02 c8 9a 35 59 45 aa 7b 84 e5 66 67 9d ae 95 96 9b 9b 5c a9 4e 49 6d 7e 54 13 94 6d b5 c4 85 74 33 f7 94 56 54 5c 2d 54 b3 56 55 49 8c 48 4f 1e 6d a3 36 aa 79 2b 4d 99 e5 50 d5 c8 b2 46 95 25 91 33 cb 1e 55 35 67 dc 72 47 70 9d 94 79 49 0c de 76 40 44 05 36 24 d5 0d 2c 33 51 93 99 0f 93 94 41 75 96 d9 ec 6e 07 e0 01" -o hello.wav
```

The library produces 16 bit signed samples at an 8kHz sampling rate.

## Encoding

The speakie_enc program is a simple utility for encoding speech into LPC bitstreams. It is strongly inspired by [BlueWizard].

[Texas Instruments LPC Speech Chips]: https://en.wikipedia.org/wiki/Texas_Instruments_LPC_Speech_Chips
[Echo II]: https://en.wikipedia.org/wiki/Echo_II_(expansion_card)
[MAME]: https://github.com/mamedev/mame
[devices/sound/tms5220.cpp]: https://github.com/mamedev/mame/blob/master/src/devices/sound/tms5220.cpp
[Buzzer Studio]: https://buzzer-studio.atomic14.com/
[BlueWizard]: https://github.com/patrick99e99/BlueWizard
[TSP50C0X/1X Family Speech Synthesizer Design Manual]: https://www.ti.com/lit/ml/spss011d/spss011d.pdf
[Chipspeech diary, part 2]: https://ploguechipsounds.blogspot.com/2014/12/chipspeech-diary-part-2.html
