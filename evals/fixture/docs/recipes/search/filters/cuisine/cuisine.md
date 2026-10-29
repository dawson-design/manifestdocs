---
id: fd-7ca0fb
title: "Cuisine filter"
status: approved
summary: "Limits search to one cuisine with `--cuisine`."
---

# Cuisine filter

## Requirements

### Requirement: One cuisine

`cookbook search --cuisine NAME` SHALL keep recipes whose cuisine is NAME, ignoring case.

#### Scenario: Indian recipes

- WHEN a reader searches with `--cuisine indian`
- THEN the results are Chicken tikka masala and Red lentil dal

#### Scenario: Mixed case

- WHEN a reader searches with `--cuisine Italian`
- THEN the results are Pasta al pomodoro and Mushroom risotto
