---
title: Image Showcase
author: SilkPrint Test Suite
---

# Images

## Basic Image

![A sample photograph](./img/photo.png)

## Image with Title Attribute

![Architecture diagram](./img/architecture.svg "SilkPrint Pipeline Architecture")

## Reference-Style Image

![Logo][logo]

[logo]: ./img/silkprint-logo.png "SilkPrint Logo"

## Image as Figure with Caption

The alt text serves as the figure caption when the image is rendered as a block element:

![Figure 1: Theme color resolution pipeline showing the three-layer token hierarchy from primitives through semantic to component tokens.](./img/color-resolution.png)

## Missing Image (Graceful Handling)

This image path does not exist and should be handled gracefully:

![This image is missing](./img/nonexistent-file.png)

## Various Formats

![PNG image](./img/sample.png)

![JPEG photo](./img/sample.jpg)

![SVG diagram](./img/diagram.svg)

## Inline Image in Text

Here is an icon ![small icon](./img/icon.png) embedded within a paragraph of text.
It should render inline at an appropriate size.

## Image with Long Alt Text

![This is a very long alt text that describes the image in great detail for accessibility purposes. It includes information about the visual content, the context in which it appears, and any relevant details that a screen reader user would need to understand the image.](./img/detailed.png)
