Network testing system written on Rust.
It consists of 3 major components:
- Input modules
- Processing modules
- Output modules

Input modules generate probes with the following attributes:
- Name: string
- Labels: a set of KV (string, string) labels for probe routing
- Values: a set of KV (string, float) values, that hold actual measurements

After probe being generated by input module, it goes to routing engine. Routing engine look for processing module, that has matches for **labels** and **values** and send probe to it. After processing, probes ingests back into routing engine. If no processing modules matches the probe, routing engine look for output module, that matches **labels** and send probe to it. If no output module matches the probe, routing engine drops probe.
Route selection process for processing modules:
- All label **keys** in the probe should exist in the match section of a processing module
- Corresponding label **values** in the probe should match regex in the match section of a processing module
- **Key** of the value in the probe should exist in the match section of a processing module

Route selection process for output modules:
- All label **keys** in the probe should exist in the match section of a processing module
- Corresponding label **values** in the probe should match regex in the match section of a processing module

List of input modules:
- **pinger** regular ICMP echo request
Config template:
```
  <probe name>:
    addr: <target address>
    check: icmp
    interval: <interval between pings (seconds)>
    config:
      mtu: <payload size (bytes)>
      source_ip: <source ip address>
    labels:
      <label name>: <label value>
      ...
```
Output values: **rtt**, **loss**. Additional labels: none.

- **mtu_pinger** ICMP echo with a set of MTU values.
```
  <probe name>:
    addr: <target address>
    check: mtu_icmp
    interval: <interval between pings (seconds)>
    config:
      mtu:
      - <mtu1>
      - <mtu2>
      - <mtu3>
      source_ip: <source ip address>
    labels:
      <label name>: <label value>
      ...
```
Output values: **rtt**, **loss**. Additional label: **mtu**.

- **syn** TCP SYN ping. Sends TCP SYN packet, waits for TCP SYN-ACK.
```
  <probe name>:
    addr: <target address>
    check: syn
    interval: <interval between pings (seconds)>
    config:
      port: <destination port>
      source_ip: <source ip address>
    labels:
      <label name>: <label value>
      ...
```
Output values: **rtt**, **loss**. Additional labels: none

- **tcp_connect** TCP CONNECT ping. Establish TCP connection.
```
  <probe name>:
    addr: <target address>:<target port>
    check: tcp_connect
    interval: <interval between pings (seconds)>
    config:
      timeout: <timeout for connection>
    labels:
      <label name>: <label value>
      ...
```
Output values: **rtt**, **loss**. Additional labels: none

- **udp_server** UDP receiver. Listen for UDP packets and mirror them back.
```
  <probe name>:
    addr: <local address>:<local port>
    check: udp_server
    interval: <ignored>
    config: {}
    labels:
      <label name>: <label value>
      ...
```
Output values: **<sender address>:<sender port>**. Additional labels: none

- **UDP client** UDP client. Send UDP packet and wait them mirrored back.
```
  <probe name>:
    addr: <target address>:<target port>
    check: udp_client
    interval: <interval between pings (seconds)>
    config:
      timeout: <timeout>
      source: <local address>:<local port>
    labels:
      <label name>: <label value>
      ...
```
Output values: **rtt**, **loss**. Additional labels: none

List of processing modules:
- Stats. Wait for a number of probes and generates stats. Possible stats to emit: **avg, low, high, sum**. If keep_name is **true** original value name is saved in labels as **value** = <name>
```
- process_name: stats_count
  config:
    interval: <number of probes to process>
  labels_to_add:
    <label name>: <label valoe>
  match_labels:
    <label name>:
    - <regex of values>
    ...
  match_value: <value to process>
  values:
  - <stat to emit>
  keep_name: <save original value name in the label. bool>
```
Output values: listed in **values**. Additional labels: **value** if **keep_name** is set.

- Time stats. Accumulate probes for **interval** of time and emit stats. Possible stats to emit: **avg, low, high, sum**. If keep_name is **true** original value name is saved in labels as **value** = <name>
```
- process_name: stats_time
  config:
    interval: <time interval (seconds)>
  labels_to_add:
    <label name>: <label valoe>
  match_labels:
    <label name>:
    - <regex of values>
    ...
  match_value: <value to process>
  values:
  - <stat to emit>
  keep_name: <save original value name in the label. bool>
```
Output values: listed in **values**. Additional labels: **value** if **keep_name** is set.

- Histogram. Wait for a number of probes and generates stats. Generate a set of values for an **interval** of time and emit them as a single probe.
```
- process_name: histogram
  config:
    interval: <time interval (seconds)>
  labels_to_add:
    <label name>: <label valoe>
  match_labels:
    <label name>:
    - <regex of values>
    ...
  match_value: <value to process>
  keep_name: <save original value name in the label. bool>
```
Output values: numbered KV as they entered module, e.g. **0**: value, **1**: value, etc.  Additional labels: **value** if **keep_name** is set.

List of output modules:
- Print. Print probe to STDOUT
```
- output_name: print
  match_labels:
    <label name>:
    - <regex of values>
    ...
  config: {}
```

- Graphite. Send probes to graphite. Graphite metric format: **<prefix>.<label values>...<value name>.<value>**
```
- output_name: graphite
  match_labels:
    <label name>:
    - <regex of values>
    ...
  config:
    address: <carbon address>:<carbon port>
    prefix: <prefix for graphite metric>
    names:
    - <label to add to graphite metric>
    ...
```

- Remote. Send probe to a remote receiver vi gRPC.
```
- output_name: remote_sender
  match_labels:
    <label name>:
    - <regex of values>
    ...
  config:
    address: <remote address>
```

There are special modules: remote_input and remote_output. Remote output module sends probes to the remote input module via gRPC. With this modules one can deploy master-slave network of probes.

Config format:
```
---

hosts:
  <input name>:
    <input definition>
  ...

processes:
- <process definition>
...

outputs:
- <output definition>
...
```

Usage:
```

RUST_LOG=<loglevel> rust-checker -c/--config <config file> [-r/--remote]
```
If **remote** flag is set, start input receive module for remote probes.