# Google Summer of Code
This project has been part of [Google Summer of Code](https://summerofcode.withgoogle.com/).

It has been proposed under the following description, which gives a good high level overview of what we want to achieve:

> Tokio provides an instrumentation API using `tracing` as well as a number of instrumentation points built into Tokio itself and the Tokio ecosystem. The goal of the project is to implement a library for aggregation, metrics of said instrumentation points and a console-based UI that connects to the process, allowing users to quickly visualize, browse and debug the data.
>
> Because processes can encode structured and typed business logic with instrumentation points based on `tracing`, a domain-specific debugger built upon those can provide powerful, ad hoc tooling, e.g. filtering events by connection id, execution context etcetera. As instrumentation points of underlying libraries are collected as well, it is easy to observe their behaviour and interaction. This is an eminent advantage over traditional debuggers, where the user instead observes the implementation.

## Summary

Initial research started in the application issue [#1](https://github.com/tokio-rs/gsoc/issues/1). A dedicated [repository](https://github.com/tokio-rs/console/) was set up and code submitted via [pull requests](https://github.com/tokio-rs/console/pulls?q=is%3Apr).

All in all, we now support*:
 - UI Navigation: Arrow keys + Mouse**
 - A small, text based query DSL
   - `group_by` operator with access to event fields and their spans
   - filtering operators for event fields:
     - Equality `==`
     - `contains "<string>"`
     - `starts_with "<string>"`
     - `matches "<regex>"`
 - Subscriber implementations for remote access
   - Transport layer: gRPC / protobuf definition
   - Threaded implementation, when no tokio runtime is available
   - Tokio/Task based implementation, no additional threads
 - Filter de-/serialization with `load <name>` and `save <name>`

In summary, over the course of the three months, we've implemented a prototype that showcases the power of structured instrumentation points provided by [tracing](https://github.com/tokio-rs/tracing/). The initial set of features is pretty solid and imo (@msleepypanda) we achieved what we set out to do!

Some things did fall short though. We're still experimenting on ways to effectively present the data. The following work items will be revisited once we reach a more stable state:
 - Verification on production applications
 - Debugging/Application guides for users

\*: Some PRs are pending review atm
<br>
**: Depends on the settings of your terminal application

## More than a prototype

Working on something that tickles you curiosity and ambitions, seeing something with potential slowly but steadily reaching a usable state _really_ drives creativity. Unsursprisingly, the list of features / directions in which we want to drive `console` doesn't fall short (no specific order):

 - More/Mightier operators
 - Filter nesting, e.g. `show "filter_a" within "filter_b"`
 - Time travel debugging
 - Revise query workflow
 - Recording / Plaback capabilities
 - Overlaying traces from multiple (distributed?) applications
 - Custom filters and widgets for libraries
 - Generate diagrams, timelines from traces
 - Web frontend
 - docs: Debugging/Application guides
 - Transition to async/await

I hope to maintain/develop the console project beyond GSoC and i'm very much excited for the future!

## Get involved

If you're interested, we'd like to hear from you! Get involved in the issue tracker or ping us on [gitter](https://gitter.im/tokio-rs/tracing) (@hawkw, @msleepypanda).

## Thank you

I'm very thankful for the opportunity working together with tokio and @hawkw, sponsered by Google. The project and thorough review process has certainly improved my style and organizational abilities. If you're eligible for GSoC, i can highly recommend applying, especially at the tokio project. I sincerely hope that they'll have the opportunity to mentor students next year.