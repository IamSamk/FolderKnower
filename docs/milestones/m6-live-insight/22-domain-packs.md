# 22. Domain packs: networks, security, IoT

**Status: later.** Order requested: ML (this milestone) → networks → security → IoT.
Each pack is a YAML catalog plus a view, built like the AI pack: rules with ids,
student sentences, and links to code.

## Networks (next pack)

- **Recognize:**
  - sockets (TCP/UDP), HTTP clients and servers, gRPC, WebSockets, message brokers
    (Kafka, NATS, RabbitMQ, Redis pub/sub);
  - service discovery and retries, timeouts, circuit breakers.
- **Visualize:**
  - the request path across processes, extending the existing cross-tier edges;
  - an OSI-layer strip per call ("HTTP over TLS over TCP");
  - live sockets per process (`/proc/net/tcp`, unprivileged).

## Security

- **Recognize:**
  - auth flows (OAuth/OIDC, JWT verify and sign, sessions);
  - crypto use (AEAD and key sizes; flag ECB or MD5 for passwords);
  - secrets handling and input validation at route boundaries.
- **Visualize:** trust boundaries on the architecture view; where untrusted input enters
  and which checks it passes.

This is static teaching, not a scanner; it points to dedicated tools for audits.

## IoT

- **Recognize:**
  - MQTT/CoAP clients and topics;
  - device SDKs (ESP-IDF, Zephyr, Arduino);
  - sensor read loops and OTA update code;
  - RTOS tasks and queues, which tie back to DSA queues.
- **Visualize:** device → gateway → cloud tiers, topic graph, duty cycle and memory
  budget for microcontrollers (flash/RAM from the linker map).
