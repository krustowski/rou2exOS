ifeq (${SONAR_HOST_URL}${SONAR_TOKEN},)
sonar_check:
else
sonar_check:
	@docker run --rm \
                --dns ${DNS_NAMESERVER} \
                -e SONAR_HOST_URL="${SONAR_HOST_URL}" \
                -e SONAR_TOKEN="${SONAR_TOKEN}" \
                -v ".:/usr/src" \
                sonarsource/sonar-scanner-cli
endif

