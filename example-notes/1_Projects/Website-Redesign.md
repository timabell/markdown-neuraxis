# Project: Company Website Redesign

status:: active
priority:: high
started:: 2024-01-08
deadline:: 2024-02-28
stakeholder:: Marketing Team
budget:: $15,000
tags:: #work #design #web-development

## 🎯 Project Goal

Redesign company website to improve conversion rates, modernize design, and enhance mobile experience.

## 📊 Success Metrics

- [ ] Page load time < 3 seconds
- [ ] Mobile score > 90 on Google Lighthouse
- [ ] Conversion rate improvement > 20%
- [ ] Accessibility WCAG 2.1 AA compliant

## 🏃 Current Sprint (Week of Jan 15)

- DOING:: Implement responsive navigation menu
- DONE:: Complete wireframes for all pages
- ACTION:: Review designs with stakeholders
- ACTION:: Set up staging environment
- WAITING:: Brand guidelines from marketing
  waiting-for:: Emma from marketing
  due:: 2024-01-17

## 📋 Project Phases

### Phase 1: Discovery & Design ✅
- [x] Stakeholder interviews
- [x] Competitor analysis
- [x] User research (10 interviews)
- [x] Information architecture
- [x] Wireframes
- [x] High-fidelity mockups

### Phase 2: Development (Current)
- [x] Set up development environment
- [x] Implement design system
- [ ] Build page templates
  - [x] Homepage
  - [ ] Product pages
  - [ ] About section
  - [ ] Blog
  - [ ] Contact
- [ ] CMS integration
- [ ] Performance optimization

### Phase 3: Testing & Launch
- [ ] Cross-browser testing
- [ ] User acceptance testing
- [ ] Performance testing
- [ ] SEO audit
- [ ] Content migration
- [ ] Launch plan
- [ ] Post-launch monitoring

## 💻 Technical Stack

```yaml
Frontend:
  - Framework: Next.js 14
  - Styling: Tailwind CSS
  - Components: Radix UI
  - Animation: Framer Motion

Backend:
  - CMS: Strapi
  - Database: PostgreSQL
  - Hosting: Vercel
  - CDN: Cloudflare

Tools:
  - Design: Figma
  - Project: Linear
  - Docs: markdown-neuraxis
```

## 🐛 Issues & Blockers

- ISSUE:: Safari mobile menu animation glitch
  severity:: medium
  assigned:: self
  
- ISSUE:: Image optimization needed for hero section
  severity:: low
  solution:: Implement lazy loading and WebP format

- BLOCKER:: Need final copy for product pages
  waiting-for:: Content team
  impact:: Can't complete product templates

## 📝 Meeting Notes

### Design Review (Jan 12)
- Stakeholders loved the clean, modern approach
- Requested more prominent CTA buttons
- Want to A/B test two homepage variants
- See full notes: [[3_Resources/Meeting-Notes/2024-01-12-Design-Review]]

### Tech Planning (Jan 10)
- Decided on Next.js for better SEO
- Will use Incremental Static Regeneration
- Implement preview mode for CMS editors

## 🔗 Resources & Links

- [Figma Designs](https://figma.com/...)
- [Staging Site](https://staging.company.com)
- [Project Board](https://linear.app/...)
- [[3_Resources/Web-Performance-Checklist]]
- [[3_Resources/Design-System-Docs]]

## ⏰ Timeline

```
Jan 08-15: Discovery & Design ████████ Complete
Jan 16-31: Development Sprint 1 ████░░░░ 50%
Feb 01-14: Development Sprint 2 ░░░░░░░░ 0%
Feb 15-21: Testing & QA ░░░░░░░░ 0%
Feb 22-28: Launch Prep ░░░░░░░░ 0%
Mar 01: 🚀 Launch Day
```

## 🤔 Decisions & Rationale

- **Why Next.js?** - SEO requirements, good DX, strong ecosystem
- **Why Strapi?** - Open source, flexible, marketing team familiar
- **Why Vercel?** - Seamless Next.js integration, great performance
- **No WordPress?** - Team wants modern stack, better performance

---

*Next review: Monday standup with development team*