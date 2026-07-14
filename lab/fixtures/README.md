# Mesurer une Big Muff donnée

`muffin-unit-measurement.template.json` est le relevé à remplir pour une seule
pédale identifiée : il ne décrit pas une valeur moyenne de modèle.

1. Photographier la carte, noter révision, numéro de série, alimentation et
   température. Alimenter ensuite à 9.00 V régulés.
2. Relever les tensions DC base/collecteur/émetteur de Q1 à Q4, sans dessouder
   les transistors. Ces tensions sont les contraintes de polarisation les plus
   utiles pour ajuster le modèle.
3. Hors tension, mesurer hFE, Vbe et Cob de chaque transistor avec le même
   courant de test indiqué dans le JSON. Ne pas remplacer ces trois valeurs par
   une fourchette générique BC239/2N5088 : elles varient fortement d'un lot à
   l'autre.
4. Mesurer les deux paires de diodes à 0.1, 1 et 10 mA. Une seule tension de
   diode de multimètre ne caractérise pas le seuil dynamique.
5. Mesurer résistance totale et position de curseur à 0/25/50/75/100 % pour
   Sustain, Tone et Volume. Les potentiomètres audio ne suivent pas une loi
   linéaire et leur tolérance ne se déduit pas de la référence imprimée.
6. Pour les résistances et condensateurs critiques, lever une patte avant toute
   mesure et ajouter la valeur dans `passive_components`. Ne pas injecter les
   valeurs nominales si elles n'ont pas été mesurées.

Copier le modèle sous un nom qui identifie l'unité, par exemple
`muffin-usa-serial-1234.json`. Une fois ce fichier rempli, il pourra servir à
calibrer le modèle et le banc SPICE ; sans ces mesures, aucune « tolérance
réelle » d'une unité ne peut être affirmée scientifiquement.
