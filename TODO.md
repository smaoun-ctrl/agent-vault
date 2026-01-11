# TODO - Améliorations et Compléments

## Implémentation Cryptographique Complète

### TPM 2.0
- [x] Implémenter écriture/lecture NV Index TPM complète (structure de base, à compléter selon matériel)
- [x] Créer et gérer clé TPM persistante non exportable (structure de base)
- [ ] Implémenter politique d'authentification TPM
- [ ] Gérer rotation des clés TPM si nécessaire

### Chiffrement Secret
- [x] Implémenter déchiffrement RSA-OAEP côté agent
- [x] Générer et gérer paire de clés RSA agent (stockée dans TPM)
- [x] Implémenter signature requêtes rotation (RSA-PSS)
- [x] Vérifier signature serveur (RSA-PSS) (structure, nécessite clé publique serveur)

### Chiffrement Licence
- [x] Vérifier format token licence (version, IV, ciphertext, tag)
- [x] Implémenter AAD (Additional Authenticated Data) pour GCM
- [x] Valider intégrité avec auth_tag

## Interface de Gestion

### CLI
- [x] Implémenter CLI complète (`license-agent-cli`)
- [x] Commandes : status, rotate, invalidate, logs, metrics, degraded-mode, tpm-status, reset
- [x] Authentification par certificat (structure de base)
- [x] Authentification par token (structure de base)

### API REST (Optionnel)
- [ ] Implémenter serveur HTTP local (actix-web)
- [ ] Endpoints REST documentés
- [ ] Rate limiting
- [ ] Authentification mutuelle

## Rotation

### Améliorations
- [x] Implémenter signature réelle des requêtes rotation
- [x] Vérification signature serveur complète (structure, nécessite clé publique serveur)
- [ ] Gestion désynchronisation version
- [ ] Confirmation serveur après rotation

### Retry
- [x] Backoff exponentiel configurable
- [x] Max retries configurable
- [x] Alertes si rotation échoue

## Mode Dégradé

### Améliorations
- [x] Détection automatique indisponibilité réseau
- [x] Retry périodique rotation en mode dégradé
- [x] Alertes progressives (24h, 72h, 144h)
- [x] Désactivation automatique sur reconnexion

## Tests

### Unitaires
- [x] Tests Secret Manager (structure de base)
- [x] Tests License Validator (structure de base)
- [x] Tests Rotation Manager (structure de base)
- [x] Tests IPC Server (structure de base)
- [x] Tests Crypto Manager (tests de base)

### Intégration
- [ ] Tests end-to-end validation licence
- [ ] Tests rotation complète
- [ ] Tests mode dégradé
- [ ] Tests récupération après panne

### Sécurité
- [ ] Tests injection
- [ ] Tests extraction secret
- [ ] Tests résilience
- [ ] Tests performance

## Documentation

### Opérationnelle
- [x] Guide de déploiement
- [x] Guide de configuration
- [ ] Guide de dépannage (partiellement dans déploiement)
- [ ] Procédures de récupération

### Développement
- [ ] Documentation API interne
- [ ] Guide contribution
- [ ] Architecture détaillée code

## Monitoring

### Métriques
- [x] Exposition métriques Prometheus (métriques définies)
- [ ] Dashboard Grafana (optionnel)
- [ ] Alertes configurées
- [ ] Exposition HTTP (à ajouter si API REST activée)

### Logs
- [ ] Rotation logs (logrotate)
- [ ] Compression logs anciens
- [ ] Export logs pour analyse

## Sécurité Renforcée

### Isolation
- [ ] Seccomp BPF profiles
- [ ] AppArmor/SELinux profiles
- [ ] Namespaces Linux
- [ ] Capabilities minimales

### Mémoire
- [ ] Mlock pour pages critiques
- [ ] No-dump flag
- [ ] Zeroization vérifiée

### Intégrité
- [ ] Vérification intégrité binaire au démarrage
- [ ] Signatures cryptographiques binaires
- [ ] TPM PCR pour mesure boot

## Performance

### Optimisations
- [ ] Cache résultats validation
- [ ] Pool connexions TPM
- [ ] Pré-chargement clés
- [ ] Opérations asynchrones optimisées

## Récupération

### Backup
- [ ] Sauvegarde automatique état
- [ ] Rotation backups
- [ ] Test restauration

### Récupération
- [ ] Procédure récupération après panne TPM
- [ ] Procédure récupération corruption état
- [ ] Procédure réinitialisation complète

## Conformité

### Standards
- [ ] Conformité ISO 27001 (si applicable)
- [ ] Conformité PCI-DSS (si applicable)
- [ ] Conformité RGPD (logs)

### Audit
- [ ] Logs immutables (horodatage cryptographique)
- [ ] Export format standard
- [ ] Rétention configurable
